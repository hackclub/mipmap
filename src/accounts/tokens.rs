use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{self, Aead, Generate, KeyInit, Payload},
};
use anyhow::anyhow;
use sqlx::{PgExecutor, PgPool};

#[derive(Clone, Copy)]
pub enum Provider {
    Hca,
    Hackatime,
}

impl Provider {
    fn name(self) -> &'static str {
        match self {
            Provider::Hca => "hca",
            Provider::Hackatime => "hackatime",
        }
    }
}

fn cipher() -> anyhow::Result<Aes256Gcm> {
    let key = std::env::var("TOKEN_ENCRYPTION_KEY").map_err(|_| anyhow!("TOKEN_ENCRYPTION_KEY must be set"))?;
    let key = hex::decode(key).map_err(|_| anyhow!("TOKEN_ENCRYPTION_KEY must be 32 bytes of hex"))?;
    Aes256Gcm::new_from_slice(&key)
        .map_err(|_| anyhow!("TOKEN_ENCRYPTION_KEY must be 32 bytes of hex"))
}

pub fn check() -> anyhow::Result<()> {
    cipher().map(|_| ())
}

type TokenNonce = aead::Nonce<Aes256Gcm>;

fn aad(user_id: i32, provider: Provider) -> Vec<u8> {
    format!("mipmap:{user_id}:{}", provider.name()).into_bytes()
}

fn encrypt(cipher: &Aes256Gcm, nonce: &TokenNonce, aad: &[u8], token: &str) -> anyhow::Result<Vec<u8>> {
    cipher
        .encrypt(nonce, Payload { msg: token.as_bytes(), aad })
        .map_err(|_| anyhow!("token encryption failed"))
}

fn decrypt(cipher: &Aes256Gcm, nonce: &TokenNonce, aad: &[u8], ciphertext: &[u8]) -> anyhow::Result<String> {
    let token = cipher
        .decrypt(nonce, Payload { msg: ciphertext, aad })
        .or_else(|_| cipher.decrypt(nonce, ciphertext))
        .map_err(|_| anyhow!("token decryption failed"))?;
    Ok(String::from_utf8(token)?)
}

pub async fn store(
    executor: impl PgExecutor<'_>,
    user_id: i32,
    provider: Provider,
    token: &str,
) -> anyhow::Result<()> {
    let nonce = Nonce::generate();
    let ciphertext = encrypt(&cipher()?, &nonce, &aad(user_id, provider), token)?;
    sqlx::query(
        "INSERT INTO auth_token (user_id, provider, ciphertext, nonce) VALUES ($1, $2, $3, $4)
         ON CONFLICT (user_id, provider) DO UPDATE SET ciphertext = $3, nonce = $4",
    )
    .bind(user_id)
    .bind(provider.name())
    .bind(ciphertext)
    .bind(nonce.as_slice())
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn load(
    pool: &PgPool,
    user_id: i32,
    provider: Provider,
) -> anyhow::Result<Option<String>> {
    let Some((ciphertext, nonce)) = sqlx::query_as::<_, (Vec<u8>, Vec<u8>)>(
        "SELECT ciphertext, nonce FROM auth_token WHERE user_id = $1 AND provider = $2",
    )
    .bind(user_id)
    .bind(provider.name())
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    let nonce = Nonce::try_from(nonce.as_slice()).map_err(|_| anyhow!("bad nonce"))?;
    Ok(Some(decrypt(&cipher()?, &nonce, &aad(user_id, provider), &ciphertext)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_bound_to_user_and_provider() {
        let cipher = Aes256Gcm::new_from_slice(&[7; 32]).unwrap();
        let nonce = Nonce::generate();
        let bound = encrypt(&cipher, &nonce, &aad(1, Provider::Hca), "secret").unwrap();
        assert_eq!(decrypt(&cipher, &nonce, &aad(1, Provider::Hca), &bound).unwrap(), "secret");
        assert!(decrypt(&cipher, &nonce, &aad(2, Provider::Hca), &bound).is_err());
        assert!(decrypt(&cipher, &nonce, &aad(1, Provider::Hackatime), &bound).is_err());

        let legacy = cipher.encrypt(&nonce, b"old".as_slice()).unwrap();
        assert_eq!(decrypt(&cipher, &nonce, &aad(1, Provider::Hca), &legacy).unwrap(), "old");
    }
}
