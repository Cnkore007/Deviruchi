use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

/// 对明文密码进行 Argon2 哈希
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Failed to hash password: {}", e))?
        .to_string();
    Ok(password_hash)
}

/// 验证明文密码是否匹配存储的哈希
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(stored_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash));
        assert!(!verify_password("wrong_password", &hash));
    }

    #[test]
    fn test_different_hashes_for_same_password() {
        let password = "same_password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();
        assert_ne!(hash1, hash2);
        assert!(verify_password(password, &hash1));
        assert!(verify_password(password, &hash2));
    }
}
