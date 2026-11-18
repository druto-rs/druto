//! Password hashing drivers, automatic rehashing detection, and verification for Druto.
//!
//! Provides memory-hard hashing drivers ([`Argon2Hasher`]), compatibility drivers
//! ([`BcryptHasher`]), a unified multi-driver registry ([`HashManager`]), and instantaneous
//! test doubles ([`FakeHasher`]).

pub mod argon2;
pub mod bcrypt;
pub mod error;
pub mod fake;
pub mod hasher;
pub mod manager;

pub use crate::{
    argon2::{Argon2Config, Argon2Hasher},
    bcrypt::{BcryptConfig, BcryptHasher},
    error::HashError,
    fake::FakeHasher,
    hasher::{HashAlgorithm, Hasher},
    manager::{FakeGuard, HashManager, HashManagerBuilder},
};
