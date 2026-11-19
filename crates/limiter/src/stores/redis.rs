//! Distributed Redis rate limiter store backed by atomic Lua scripts.

use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use redis::{
    RedisCommands,
    error::RedisError,
    script::{RedisScript, ScriptContext, ScriptOutput},
};
use serde::{Deserialize, Serialize};

use crate::{
    algorithm::Algorithm,
    error::LimiterError,
    result::RateLimitResult,
    store::{BoxFuture, LimiterStore},
};

/// Default key prefix for limiter keys in Redis.
pub const DEFAULT_REDIS_KEY_PREFIX: &str = "druto:limiter:";

const LUA_RATE_LIMITER_SCRIPT: &str = r#"
local key = KEYS[1]
local block_key = KEYS[2]

local amount = tonumber(ARGV[1])
local limit = tonumber(ARGV[2])
local window_ms = tonumber(ARGV[3])
local now_ms = tonumber(ARGV[4])
local algorithm = ARGV[5]
local is_peek = tonumber(ARGV[6])

-- 1. Check if key is currently blocked
local blocked = redis.call('GET', block_key)
if blocked then
    local pttl = redis.call('PTTL', block_key)
    if pttl < 0 then pttl = window_ms end
    return {0, 0, pttl, 1}
end

-- 2. Fixed Window
if algorithm == "fixed_window" then
    local current_window = math.floor(now_ms / window_ms)
    local win_key = key .. ":" .. current_window
    local count = tonumber(redis.call('GET', win_key) or "0")
    local time_into_win = now_ms % window_ms
    local reset_after = window_ms - time_into_win

    if is_peek == 1 then
        local remaining = math.max(0, limit - count)
        local allowed = remaining > 0 and 1 or 0
        return {allowed, remaining, reset_after, 0}
    end

    if count + amount <= limit then
        count = redis.call('INCRBY', win_key, amount)
        redis.call('PEXPIRE', win_key, window_ms + 1000)
        local remaining = math.max(0, limit - count)
        return {1, remaining, reset_after, 0}
    else
        local remaining = math.max(0, limit - count)
        return {0, remaining, reset_after, 0}
    end
end

-- 3. Token Bucket
if algorithm == "token_bucket" then
    local bucket = redis.call('HMGET', key, 'tokens', 'last_refill')
    local tokens = tonumber(bucket[1] or limit)
    local last_refill = tonumber(bucket[2] or now_ms)
    local refill_rate = limit / (window_ms / 1000.0)

    local elapsed = math.max(0, (now_ms - last_refill) / 1000.0)
    tokens = math.min(limit, tokens + (elapsed * refill_rate))
    last_refill = now_ms

    local reset_after = 0
    if tokens < limit then
        reset_after = math.ceil(((limit - tokens) / refill_rate) * 1000)
    end

    if is_peek == 1 then
        local remaining = math.floor(tokens)
        local allowed = remaining >= amount and 1 or 0
        return {allowed, remaining, reset_after, 0}
    end

    if tokens >= amount then
        tokens = tokens - amount
        redis.call('HMSET', key, 'tokens', tokens, 'last_refill', last_refill)
        redis.call('PEXPIRE', key, window_ms * 2)
        return {1, math.floor(tokens), reset_after, 0}
    else
        local retry_after = math.ceil(((amount - tokens) / refill_rate) * 1000)
        return {0, math.floor(tokens), retry_after, 0}
    end
end

-- 4. Sliding Window Counter (Default)
local current_window = math.floor(now_ms / window_ms)
local time_into_win = now_ms % window_ms
local reset_after = window_ms - time_into_win

local stored = redis.call('HMGET', key, 'curr_win', 'curr_cnt', 'prev_cnt')
local c_win = tonumber(stored[1] or current_window)
local c_cnt = tonumber(stored[2] or "0")
local p_cnt = tonumber(stored[3] or "0")

if c_win == current_window then
    -- Current active window
elseif c_win == current_window - 1 then
    p_cnt = c_cnt
    c_cnt = 0
    c_win = current_window
else
    p_cnt = 0
    c_cnt = 0
    c_win = current_window
end

local weight = 1.0 - (time_into_win / window_ms)
local estimated = c_cnt + math.floor(p_cnt * weight)

if is_peek == 1 then
    local remaining = math.max(0, limit - estimated)
    local allowed = remaining > 0 and 1 or 0
    return {allowed, remaining, reset_after, 0}
end

if estimated + amount <= limit then
    c_cnt = c_cnt + amount
    redis.call('HMSET', key, 'curr_win', c_win, 'curr_cnt', c_cnt, 'prev_cnt', p_cnt)
    redis.call('PEXPIRE', key, window_ms * 2)
    local remaining = math.max(0, limit - c_cnt)
    return {1, remaining, reset_after, 0}
else
    local remaining = 0
    return {0, remaining, reset_after, 0}
end
"#;

#[derive(Serialize, Deserialize, Default)]
struct SimulationState {
    curr_win: u64,
    curr_cnt: u64,
    prev_cnt: u64,
    tokens: f64,
    last_refill: u64,
}

fn create_rate_limiter_script() -> RedisScript {
    RedisScript::new(LUA_RATE_LIMITER_SCRIPT).with_simulation(
        |keys: &[&str],
         args: &[&str],
         context: &mut dyn ScriptContext|
         -> Result<ScriptOutput, RedisError> {
            let key = keys[0];
            let block_key = keys[1];

            let amount: u64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(1);
            let limit: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100);
            let window_ms: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(60000);
            let now_ms: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
            let algorithm = args.get(4).copied().unwrap_or("sliding_window");
            let is_peek: u64 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);

            // 1. Check if blocked
            if context.exists(block_key) {
                let ttl_secs = context.ttl(block_key);
                let pttl_ms = if ttl_secs > 0 {
                    ttl_secs as u64 * 1000
                } else {
                    window_ms
                };
                return Ok(ScriptOutput::Array(vec![
                    ScriptOutput::Integer(0),
                    ScriptOutput::Integer(0),
                    ScriptOutput::Integer(pttl_ms as i64),
                    ScriptOutput::Integer(1),
                ]));
            }

            // 2. Fixed Window
            if algorithm == "fixed_window" {
                let current_window = now_ms / window_ms;
                let win_key = format!("{key}:{current_window}");
                let count: u64 = context
                    .get(&win_key)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let time_into_win = now_ms % window_ms;
                let reset_after = window_ms - time_into_win;

                if is_peek == 1 {
                    let remaining = limit.saturating_sub(count);
                    let allowed = if remaining > 0 { 1 } else { 0 };
                    return Ok(ScriptOutput::Array(vec![
                        ScriptOutput::Integer(allowed),
                        ScriptOutput::Integer(remaining as i64),
                        ScriptOutput::Integer(reset_after as i64),
                        ScriptOutput::Integer(0),
                    ]));
                }

                let (allowed, remaining) = if count.saturating_add(amount) <= limit {
                    let new_count = count.saturating_add(amount);
                    context.set_ex(
                        &win_key,
                        &new_count.to_string(),
                        (window_ms / 1000).max(1) + 1,
                    );
                    (1, limit.saturating_sub(new_count))
                } else {
                    (0, limit.saturating_sub(count))
                };

                return Ok(ScriptOutput::Array(vec![
                    ScriptOutput::Integer(allowed),
                    ScriptOutput::Integer(remaining as i64),
                    ScriptOutput::Integer(reset_after as i64),
                    ScriptOutput::Integer(0),
                ]));
            }

            // 3. Token Bucket
            if algorithm == "token_bucket" {
                let raw_state = context.get(key);
                let mut state: SimulationState = raw_state
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_else(|| SimulationState {
                        tokens: limit as f64,
                        last_refill: now_ms,
                        ..Default::default()
                    });

                let refill_rate = limit as f64 / (window_ms as f64 / 1000.0);
                let elapsed_secs = (now_ms.saturating_sub(state.last_refill)) as f64 / 1000.0;
                state.tokens = (state.tokens + elapsed_secs * refill_rate).min(limit as f64);
                state.last_refill = now_ms;

                let reset_after = if state.tokens < limit as f64 {
                    let needed = limit as f64 - state.tokens;
                    (needed / refill_rate * 1000.0) as u64
                } else {
                    0
                };

                if is_peek == 1 {
                    let remaining = state.tokens.floor() as u64;
                    let allowed = if remaining >= amount { 1 } else { 0 };
                    return Ok(ScriptOutput::Array(vec![
                        ScriptOutput::Integer(allowed),
                        ScriptOutput::Integer(remaining as i64),
                        ScriptOutput::Integer(reset_after as i64),
                        ScriptOutput::Integer(0),
                    ]));
                }

                let (allowed, after) = if state.tokens >= amount as f64 {
                    state.tokens -= amount as f64;
                    if let Ok(serialized) = serde_json::to_string(&state) {
                        context.set_ex(key, &serialized, (window_ms / 1000).max(1) * 2);
                    }
                    (1, reset_after)
                } else {
                    let retry_after =
                        ((amount as f64 - state.tokens) / refill_rate * 1000.0) as u64;
                    (0, retry_after)
                };

                return Ok(ScriptOutput::Array(vec![
                    ScriptOutput::Integer(allowed),
                    ScriptOutput::Integer(state.tokens.floor() as i64),
                    ScriptOutput::Integer(after as i64),
                    ScriptOutput::Integer(0),
                ]));
            }

            // 4. Sliding Window Counter
            let current_window = now_ms / window_ms;
            let time_into_win = now_ms % window_ms;
            let reset_after = window_ms - time_into_win;

            let raw_state = context.get(key);
            let mut state: SimulationState = raw_state
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_else(|| SimulationState {
                    curr_win: current_window,
                    curr_cnt: 0,
                    prev_cnt: 0,
                    ..Default::default()
                });

            if state.curr_win == current_window {
                // Same active window
            } else if state.curr_win + 1 == current_window {
                state.prev_cnt = state.curr_cnt;
                state.curr_cnt = 0;
                state.curr_win = current_window;
            } else {
                state.prev_cnt = 0;
                state.curr_cnt = 0;
                state.curr_win = current_window;
            }

            let weight = 1.0 - (time_into_win as f64 / window_ms as f64);
            let estimated = state.curr_cnt as f64 + (state.prev_cnt as f64 * weight);

            if is_peek == 1 {
                let remaining = limit.saturating_sub(estimated.floor() as u64);
                let allowed = if remaining > 0 { 1 } else { 0 };
                return Ok(ScriptOutput::Array(vec![
                    ScriptOutput::Integer(allowed),
                    ScriptOutput::Integer(remaining as i64),
                    ScriptOutput::Integer(reset_after as i64),
                    ScriptOutput::Integer(0),
                ]));
            }

            if estimated + amount as f64 <= limit as f64 {
                state.curr_cnt = state.curr_cnt.saturating_add(amount);
                if let Ok(serialized) = serde_json::to_string(&state) {
                    context.set_ex(key, &serialized, (window_ms / 1000).max(1) * 2);
                }
                let remaining = limit.saturating_sub(state.curr_cnt);
                Ok(ScriptOutput::Array(vec![
                    ScriptOutput::Integer(1),
                    ScriptOutput::Integer(remaining as i64),
                    ScriptOutput::Integer(reset_after as i64),
                    ScriptOutput::Integer(0),
                ]))
            } else {
                Ok(ScriptOutput::Array(vec![
                    ScriptOutput::Integer(0),
                    ScriptOutput::Integer(0),
                    ScriptOutput::Integer(reset_after as i64),
                    ScriptOutput::Integer(0),
                ]))
            }
        },
    )
}

/// Distributed Redis-backed rate limiter store.
#[derive(Clone)]
pub struct RedisLimiterStore<C> {
    client: Arc<C>,
    prefix: String,
    script: RedisScript,
}

impl<C: RedisCommands + 'static> RedisLimiterStore<C> {
    /// Creates a new `RedisLimiterStore` with the default prefix (`druto:limiter:`).
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            prefix: DEFAULT_REDIS_KEY_PREFIX.to_string(),
            script: create_rate_limiter_script(),
        }
    }

    /// Creates a new `RedisLimiterStore` with a customized key prefix.
    pub fn with_prefix(client: Arc<C>, prefix: impl Into<String>) -> Self {
        Self {
            client,
            prefix: prefix.into(),
            script: create_rate_limiter_script(),
        }
    }

    fn key_for(&self, key: &str) -> String {
        format!("{}{key}", self.prefix)
    }

    fn block_key_for(&self, key: &str) -> String {
        format!("{}{key}:blocked", self.prefix)
    }

    fn parse_script_output(
        output: ScriptOutput,
        limit: u64,
    ) -> Result<RateLimitResult, LimiterError> {
        match output {
            ScriptOutput::Array(items) if items.len() >= 4 => {
                let allowed = match items[0] {
                    ScriptOutput::Integer(val) => val == 1,
                    _ => false,
                };
                let remaining = match items[1] {
                    ScriptOutput::Integer(val) => val.max(0) as u64,
                    _ => 0,
                };
                let reset_after_ms = match items[2] {
                    ScriptOutput::Integer(val) => val.max(0) as u64,
                    _ => 0,
                };
                let is_blocked = match items[3] {
                    ScriptOutput::Integer(val) => val == 1,
                    _ => false,
                };

                let reset_after = Duration::from_millis(reset_after_ms);
                let retry_after = if !allowed || is_blocked {
                    Some(reset_after)
                } else {
                    None
                };

                Ok(RateLimitResult::new(
                    limit,
                    remaining,
                    reset_after,
                    allowed,
                    retry_after,
                ))
            }
            _ => Err(LimiterError::Store(
                "Malformed Lua script output from Redis".to_string(),
            )),
        }
    }
}

impl<C: RedisCommands + 'static> LimiterStore for RedisLimiterStore<C> {
    fn consume<'a>(
        &'a self,
        key: &'a str,
        amount: u64,
        limit: u64,
        window: Duration,
        algorithm: Algorithm,
    ) -> BoxFuture<'a, Result<RateLimitResult, LimiterError>> {
        Box::pin(async move {
            let main_key = self.key_for(key);
            let block_key = self.block_key_for(key);

            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let window_ms = window.as_millis().max(1) as u64;

            let amount_str = amount.to_string();
            let limit_str = limit.to_string();
            let window_str = window_ms.to_string();
            let now_str = now_ms.to_string();
            let algo_str = algorithm.to_string();

            let keys = [main_key.as_str(), block_key.as_str()];
            let args = [
                amount_str.as_str(),
                limit_str.as_str(),
                window_str.as_str(),
                now_str.as_str(),
                algo_str.as_str(),
                "0",
            ];

            let output = self
                .client
                .eval_script(&self.script, &keys, &args)
                .await
                .map_err(|error| LimiterError::Store(format!("Redis execution failed: {error}")))?;

            Self::parse_script_output(output, limit)
        })
    }

    fn peek<'a>(
        &'a self,
        key: &'a str,
        limit: u64,
        window: Duration,
        algorithm: Algorithm,
    ) -> BoxFuture<'a, Result<RateLimitResult, LimiterError>> {
        Box::pin(async move {
            let main_key = self.key_for(key);
            let block_key = self.block_key_for(key);

            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let window_ms = window.as_millis().max(1) as u64;

            let limit_str = limit.to_string();
            let window_str = window_ms.to_string();
            let now_str = now_ms.to_string();
            let algo_str = algorithm.to_string();

            let keys = [main_key.as_str(), block_key.as_str()];
            let args = [
                "1",
                limit_str.as_str(),
                window_str.as_str(),
                now_str.as_str(),
                algo_str.as_str(),
                "1",
            ];

            let output = self
                .client
                .eval_script(&self.script, &keys, &args)
                .await
                .map_err(|error| LimiterError::Store(format!("Redis peek failed: {error}")))?;

            Self::parse_script_output(output, limit)
        })
    }

    fn reset<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), LimiterError>> {
        Box::pin(async move {
            let main_key = self.key_for(key);
            let block_key = self.block_key_for(key);

            self.client
                .del(&main_key)
                .await
                .map_err(|error| LimiterError::Store(format!("Redis DEL failed: {error}")))?;
            self.client
                .del(&block_key)
                .await
                .map_err(|error| LimiterError::Store(format!("Redis DEL failed: {error}")))?;
            Ok(())
        })
    }

    fn block<'a>(
        &'a self,
        key: &'a str,
        duration: Duration,
    ) -> BoxFuture<'a, Result<(), LimiterError>> {
        Box::pin(async move {
            let block_key = self.block_key_for(key);
            let seconds = duration.as_secs().max(1);

            self.client
                .set_ex(&block_key, "1", seconds)
                .await
                .map_err(|error| LimiterError::Store(format!("Redis block failed: {error}")))?;
            Ok(())
        })
    }
}
