use hash::{
    Argon2Config, Argon2Hasher, BcryptConfig, BcryptHasher, FakeHasher, HashAlgorithm, HashError,
    HashManager, Hasher,
};

#[test]
fn test_fake_hasher_roundtrip_and_mismatch() {
    let hasher = FakeHasher::new();
    assert_eq!(hasher.algorithm(), HashAlgorithm::Fake);

    let password = "super-secret-password";
    let hash = hasher.make(password).expect("make must succeed");

    assert!(hash.starts_with("$fake$"));
    assert_eq!(hash, "$fake$super-secret-password");
    assert!(hasher.is_valid_hash(&hash));

    let verify_success = hasher.verify(password, &hash).expect("verify must succeed");
    assert!(verify_success);

    let verify_failure = hasher
        .verify("wrong-password", &hash)
        .expect("verify must succeed on mismatch");
    assert!(!verify_failure);

    // FakeHasher also accepts plain strings for backwards compatibility in tests
    let plain_verify = hasher
        .verify(password, password)
        .expect("plain verify must succeed");
    assert!(plain_verify);

    assert!(!hasher.needs_rehash(&hash));
    assert!(hasher.needs_rehash("raw_unhashed_password"));
}

#[test]
fn test_argon2_hasher_roundtrip_and_mismatch() {
    // Use lower cost parameters for fast integration test execution
    let config = Argon2Config::new()
        .memory(4096)
        .iterations(1)
        .parallelism(1);
    let hasher = Argon2Hasher::new(config).expect("argon2 hasher initialization must succeed");
    assert_eq!(hasher.algorithm(), HashAlgorithm::Argon2id);

    let password = "druto-secure-pass";
    let hash = hasher.make(password).expect("make must succeed");

    assert!(hash.starts_with("$argon2id$"));
    assert!(hasher.is_valid_hash(&hash));

    let valid = hasher.verify(password, &hash).expect("verify must succeed");
    assert!(valid);

    let invalid = hasher
        .verify("incorrect-pass", &hash)
        .expect("verify must succeed with false for wrong password");
    assert!(!invalid);
}

#[test]
fn test_argon2_malformed_hash_handling() {
    let hasher = Argon2Hasher::default();

    let malformed_result = hasher.verify("password", "invalid_phc_format");
    assert!(malformed_result.is_err());
    let err = malformed_result.expect_err("error must exist");
    assert!(err.is_invalid_hash());

    let truncated_result = hasher.verify("password", "$argon2id$v=19$m=65536");
    assert!(truncated_result.is_err());
    assert!(
        truncated_result
            .expect_err("error must exist")
            .is_invalid_hash()
    );
}

#[test]
fn test_argon2_needs_rehash() {
    let low_config = Argon2Config::new()
        .memory(4096)
        .iterations(1)
        .parallelism(1);
    let low_hasher = Argon2Hasher::new(low_config).expect("low hasher init failed");
    let low_hash = low_hasher.make("password").expect("make failed");

    // Identical parameters should not trigger rehash
    assert!(!low_hasher.needs_rehash(&low_hash));

    // Hasher with higher memory requirement must trigger rehash
    let high_memory_config = Argon2Config::new()
        .memory(8192)
        .iterations(1)
        .parallelism(1);
    let high_memory_hasher =
        Argon2Hasher::new(high_memory_config).expect("high memory init failed");
    assert!(high_memory_hasher.needs_rehash(&low_hash));

    // Hasher with higher iterations must trigger rehash
    let high_iterations_config = Argon2Config::new()
        .memory(4096)
        .iterations(2)
        .parallelism(1);
    let high_iterations_hasher =
        Argon2Hasher::new(high_iterations_config).expect("high iterations init failed");
    assert!(high_iterations_hasher.needs_rehash(&low_hash));

    // Hasher with higher parallelism must trigger rehash
    let high_parallelism_config = Argon2Config::new()
        .memory(4096)
        .iterations(1)
        .parallelism(2);
    let high_parallelism_hasher =
        Argon2Hasher::new(high_parallelism_config).expect("high parallelism init failed");
    assert!(high_parallelism_hasher.needs_rehash(&low_hash));

    // Malformed hash or cross-algorithm must trigger rehash
    assert!(low_hasher.needs_rehash("not_a_valid_hash"));
    assert!(low_hasher.needs_rehash("$2b$12$someotheralgorithmhash"));
}

#[test]
fn test_bcrypt_hasher_roundtrip_and_mismatch() {
    // Use minimum cost for fast test execution
    let config = BcryptConfig::new().rounds(4).expect("rounds must be valid");
    let hasher = BcryptHasher::new(config).expect("bcrypt hasher init failed");
    assert_eq!(hasher.algorithm(), HashAlgorithm::Bcrypt);

    let password = "bcrypt-password-test";
    let hash = hasher.make(password).expect("bcrypt make must succeed");

    assert!(hash.starts_with("$2b$04$"));
    assert!(hasher.is_valid_hash(&hash));

    let valid = hasher.verify(password, &hash).expect("verify must succeed");
    assert!(valid);

    let invalid = hasher
        .verify("wrong-password", &hash)
        .expect("verify must return false");
    assert!(!invalid);
}

#[test]
fn test_bcrypt_malformed_hash_handling() {
    let hasher = BcryptHasher::default();

    let malformed_result = hasher.verify("password", "invalid_bcrypt_hash");
    assert!(malformed_result.is_err());
    assert!(
        malformed_result
            .expect_err("error must exist")
            .is_invalid_hash()
    );

    let invalid_prefix = hasher.verify("password", "$3a$12$somegarbagehashformat");
    assert!(invalid_prefix.is_err());
    assert!(
        invalid_prefix
            .expect_err("error must exist")
            .is_invalid_hash()
    );
}

#[test]
fn test_bcrypt_needs_rehash() {
    let low_config = BcryptConfig::new().rounds(4).expect("rounds must be valid");
    let low_hasher = BcryptHasher::new(low_config).expect("hasher init failed");
    let hash = low_hasher.make("password").expect("make failed");

    // Same cost -> no rehash
    assert!(!low_hasher.needs_rehash(&hash));

    // Higher cost -> rehash needed
    let higher_config = BcryptConfig::new().rounds(6).expect("rounds must be valid");
    let higher_hasher = BcryptHasher::new(higher_config).expect("hasher init failed");
    assert!(higher_hasher.needs_rehash(&hash));

    // Malformed or foreign algorithm -> rehash needed
    assert!(low_hasher.needs_rehash("malformed"));
    assert!(low_hasher.needs_rehash("$argon2id$v=19$m=65536,t=3,p=4$salt$hash"));
}

#[test]
fn test_hash_manager_facade_and_drivers() {
    let manager = HashManager::default();

    // Default driver is Argon2id
    assert_eq!(
        manager.default_hasher().algorithm(),
        HashAlgorithm::Argon2id
    );

    // Resolving named drivers
    let bcrypt_driver = manager.driver("bcrypt").expect("bcrypt driver must exist");
    assert_eq!(bcrypt_driver.algorithm(), HashAlgorithm::Bcrypt);

    let fake_driver = manager.use_driver("fake").expect("fake driver must exist");
    assert_eq!(fake_driver.algorithm(), HashAlgorithm::Fake);

    let missing = manager.driver("non_existent");
    assert!(missing.is_err());
    assert!(missing.err().expect("err must exist").is_driver_not_found());
}

#[test]
fn test_hash_manager_fake_guard_lifecycle() {
    let manager = HashManager::default();

    let password = "temporary-secret";

    // Normal execution uses Argon2id
    assert_eq!(
        manager.default_hasher().algorithm(),
        HashAlgorithm::Argon2id
    );

    {
        let fake_guard = manager.fake();
        assert_eq!(manager.default_hasher().algorithm(), HashAlgorithm::Fake);

        let fake_hash = manager.make(password).expect("fake make must succeed");
        assert!(fake_hash.starts_with("$fake$"));
        assert!(
            manager
                .verify(password, &fake_hash)
                .expect("verify must succeed")
        );

        let keep_guard_alive = fake_guard;
        drop(keep_guard_alive);
    }

    // After fake_guard drops, normal driver is restored
    assert_eq!(
        manager.default_hasher().algorithm(),
        HashAlgorithm::Argon2id
    );
}

#[test]
fn test_hash_manager_standalone_fake_manager() {
    let fake_manager = HashManager::fake_manager();
    assert_eq!(
        fake_manager.default_hasher().algorithm(),
        HashAlgorithm::Fake
    );

    let password = "fast-test-secret";
    let hash = fake_manager.make(password).expect("make must succeed");
    assert!(hash.starts_with("$fake$"));
    assert!(
        fake_manager
            .verify(password, &hash)
            .expect("verify must succeed")
    );
}

#[test]
fn test_hash_manager_custom_builder() {
    let custom_argon = Argon2Hasher::new(
        Argon2Config::new()
            .memory(4096)
            .iterations(1)
            .parallelism(1),
    )
    .expect("custom argon init failed");

    let manager = HashManager::builder()
        .driver("custom_argon", custom_argon)
        .driver("test_fake", FakeHasher::new())
        .default_driver("test_fake")
        .build()
        .expect("builder must succeed");

    assert_eq!(manager.default_hasher().algorithm(), HashAlgorithm::Fake);
    let resolved = manager
        .driver("custom_argon")
        .expect("custom driver must be present");
    assert_eq!(resolved.algorithm(), HashAlgorithm::Argon2id);

    // Missing default driver in builder must fail
    let invalid_builder = HashManager::builder()
        .driver("driver_a", FakeHasher::new())
        .default_driver("driver_b")
        .build();
    assert!(invalid_builder.is_err());
}

#[test]
fn test_error_display_and_helpers() {
    let err_invalid = HashError::InvalidHash("bad format".to_string());
    assert!(err_invalid.is_invalid_hash());
    assert_eq!(err_invalid.to_string(), "invalid hash string: bad format");

    let err_params = HashError::InvalidParameters("too low".to_string());
    assert!(err_params.is_invalid_parameters());
    assert_eq!(err_params.to_string(), "invalid hash parameters: too low");

    let err_driver = HashError::DriverNotFound("missing".to_string());
    assert!(err_driver.is_driver_not_found());
    assert_eq!(err_driver.to_string(), "hash driver not found: 'missing'");

    let err_internal = HashError::Internal("failed".to_string());
    assert!(err_internal.is_internal());
    assert_eq!(err_internal.to_string(), "internal hashing failure: failed");
}

#[test]
fn test_argon2_config_edge_cases_and_custom_variants() {
    use argon2::Algorithm as ArgonAlgorithm;

    let config = Argon2Config::new()
        .memory(1024)
        .iterations(2)
        .parallelism(2);
    assert_eq!(config.memory, 1024);
    assert_eq!(config.iterations, 2);
    assert_eq!(config.parallelism, 2);

    let hasher = Argon2Hasher::new(config.clone()).expect("init failed");
    assert_eq!(hasher.config(), &config);

    // Custom algorithm variant (e.g. Argon2d)
    let mut custom_config = config;
    custom_config.algorithm = ArgonAlgorithm::Argon2d;
    let custom_hasher = Argon2Hasher::new(custom_config).expect("custom variant init");
    assert_eq!(
        custom_hasher.algorithm(),
        HashAlgorithm::Custom("argon2d".to_string())
    );

    let argon2d_hash = custom_hasher.make("password").expect("make argon2d");
    assert!(custom_hasher.is_valid_hash(&argon2d_hash));

    // Invalid parameters (memory too small for parallelism)
    let invalid_config = Argon2Config::new().memory(8).parallelism(16);
    let invalid_params = invalid_config.to_params();
    assert!(invalid_params.is_err());
    assert!(
        invalid_params
            .expect_err("must fail")
            .is_invalid_parameters()
    );

    let invalid_init = Argon2Hasher::new(invalid_config);
    assert!(invalid_init.is_err());

    // Needs rehash when algorithm differs or version is legacy
    let default_hasher = Argon2Hasher::default();
    assert!(default_hasher.needs_rehash(&argon2d_hash));

    let valid_argon2id_hash = default_hasher.make("password").expect("make valid");
    let v16_hash = valid_argon2id_hash.replace("$v=19$", "$v=16$");
    assert!(default_hasher.needs_rehash(&v16_hash));

    let no_v_hash = valid_argon2id_hash.replace("$v=19$", "");
    assert!(default_hasher.needs_rehash(&no_v_hash));

    // is_valid_hash returns false on invalid phc or foreign algorithm
    assert!(!default_hasher.is_valid_hash("garbage_hash"));
    assert!(!default_hasher.is_valid_hash(&argon2d_hash));
}

#[test]
fn test_bcrypt_config_bounds_and_invalid_hash_formats() {
    // Bounds checking on rounds
    let low_err = BcryptConfig::new().rounds(3);
    assert!(low_err.is_err());
    assert!(low_err.expect_err("too low").is_invalid_parameters());

    let high_err = BcryptConfig::new().rounds(32);
    assert!(high_err.is_err());
    assert!(high_err.expect_err("too high").is_invalid_parameters());

    let valid_config = BcryptConfig::new().rounds(10).expect("valid rounds");
    let hasher = BcryptHasher::new(valid_config.clone()).expect("hasher init");
    assert_eq!(hasher.config(), &valid_config);

    // BcryptHasher::new with invalid rounds
    let invalid_rounds_hasher = BcryptHasher::new(BcryptConfig { rounds: 50 });
    assert!(invalid_rounds_hasher.is_err());
    assert!(
        invalid_rounds_hasher
            .expect_err("50 is invalid")
            .is_invalid_parameters()
    );

    // Needs rehash edge cases: wrong prefix version, non-number cost, or cost not allowed
    assert!(hasher.needs_rehash("$2z$10$abcdefghijklmnopqrstuvwxyz123456789012345678901234567"));
    assert!(hasher.needs_rehash("$2b$xx$abcdefghijklmnopqrstuvwxyz123456789012345678901234567"));

    // Bcrypt cost out of bounds in verify (CostNotAllowed error)
    let bad_cost_res = hasher.verify(
        "password",
        "$2b$02$abcdefghijklmnopqrstuvwxyz123456789012345678901234567",
    );
    assert!(bad_cost_res.is_err());
    assert!(
        bad_cost_res
            .expect_err("cost out of bounds")
            .is_invalid_hash()
    );

    // is_valid_hash false branches
    assert!(!hasher.is_valid_hash(""));
    assert!(!hasher.is_valid_hash("$2z$10$abcdefghijklmnopqrstuvwxyz123456789012345678901234567"));
    assert!(!hasher.is_valid_hash("$2b$xx$abcdefghijklmnopqrstuvwxyz123456789012345678901234567"));
    assert!(!hasher.is_valid_hash("$2b$10$short"));
}

#[test]
fn test_hash_algorithm_display() {
    assert_eq!(format!("{}", HashAlgorithm::Argon2id), "argon2id");
    assert_eq!(format!("{}", HashAlgorithm::Bcrypt), "bcrypt");
    assert_eq!(format!("{}", HashAlgorithm::Fake), "fake");
    assert_eq!(
        format!("{}", HashAlgorithm::Custom("scrypt".to_string())),
        "scrypt"
    );
}

#[test]
fn test_hash_manager_extended_methods_and_fake_driver_resolution() {
    use std::sync::Arc;

    let manager = HashManager::default();

    // needs_rehash and is_valid_hash facade methods
    let password = "facade-test-password";
    let hash = manager.make(password).expect("make must succeed");
    assert!(!manager.needs_rehash(&hash));
    assert!(manager.is_valid_hash(&hash));

    // use_driver alias
    let used_driver = manager
        .use_driver("bcrypt")
        .expect("use_driver must succeed");
    assert_eq!(used_driver.algorithm(), HashAlgorithm::Bcrypt);

    // restore method
    let guard = manager.fake();
    assert_eq!(manager.default_hasher().algorithm(), HashAlgorithm::Fake);
    manager.restore();
    assert_eq!(
        manager.default_hasher().algorithm(),
        HashAlgorithm::Argon2id
    );

    // driver resolution without fake for non-existent driver
    let nonexistent_real = manager.driver("unknown_driver_name");
    assert!(nonexistent_real.is_err());
    assert!(
        nonexistent_real
            .err()
            .expect("must fail")
            .is_driver_not_found()
    );

    // driver resolution under fake:
    // When fake is active, valid driver names return fake, but non-existent drivers return DriverNotFound!
    let guard2 = manager.fake();
    let bcrypt_fake = manager.driver("bcrypt").expect("valid driver must succeed");
    assert_eq!(bcrypt_fake.algorithm(), HashAlgorithm::Fake);

    let nonexistent_under_fake = manager.driver("non_existent_typo");
    assert!(nonexistent_under_fake.is_err());
    let err = nonexistent_under_fake.err().expect("typo must fail");
    assert!(err.is_driver_not_found());
    drop(guard2);
    drop(guard);

    // builder default driver fallback when not explicitly specified
    let default_fallback_manager = HashManager::builder()
        .driver_arc("argon2id", Arc::new(Argon2Hasher::default()))
        .build()
        .expect("builder fallback to argon2id default");
    assert_eq!(
        default_fallback_manager.default_hasher().algorithm(),
        HashAlgorithm::Argon2id
    );

    // argon2 hash without version in needs_rehash
    let argon2_hasher = Argon2Hasher::default();
    assert!(argon2_hasher.needs_rehash("$argon2id$m=65536,t=3,p=4$c29tZXNhbHQ$c29tZWhhc2g"));

    // driver_arc builder method
    let custom_arc: Arc<dyn Hasher> = Arc::new(FakeHasher::new());
    let arc_manager = HashManager::builder()
        .driver_arc("custom_fake", custom_arc)
        .default_driver("custom_fake")
        .build()
        .expect("builder with driver_arc must succeed");
    assert_eq!(
        arc_manager.default_hasher().algorithm(),
        HashAlgorithm::Fake
    );
}
