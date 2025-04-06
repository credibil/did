//! Key management

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use base64ct::{Base64UrlUnpadded, Encoding};
use credibil_infosec::{Algorithm, PublicKeyJwk, Signer};
use ed25519_dalek::{Signer as _, SigningKey};
use rand::rngs::OsRng;

#[derive(Clone, Debug)]
pub struct Keyring {
    keys: Arc<Mutex<HashMap<String, String>>>,
    next_keys: Arc<Mutex<HashMap<String, String>>>,
    verification_method: Arc<Mutex<String>>,
}

impl Keyring {
    // Create a new keyring pre-populated with some keys.
    #[must_use]
    pub fn new() -> Self {
        Self {
            keys: Arc::new(Mutex::new(HashMap::new())),
            next_keys: Arc::new(Mutex::new(HashMap::new())),
            verification_method: Arc::new(Mutex::new(String::new())),
        }
    }

    // Set the verification method for the keyring.
    pub fn set_verification_method(&self, vm: impl ToString) -> anyhow::Result<()> {
        let mut verification_method = self.verification_method.lock().map_err(|_| {
            anyhow!("failed to lock verification method mutex")
        })?;
        *verification_method = vm.to_string();
        Ok(())
    }

    // Add a newly generated key to the keyring and corresponding next key.
    pub fn add_key(&self, id: impl ToString) -> anyhow::Result<()> {
        let mut keys = self.keys.lock().map_err(|_| {
            anyhow!("failed to lock keys mutex")
        })?;
        let signing_key = SigningKey::generate(&mut OsRng);
        let key = Base64UrlUnpadded::encode_string(signing_key.as_bytes());
        keys.insert(id.to_string(), key);

        let mut next_keys = self.next_keys.lock().unwrap();
        let next_signing_key = SigningKey::generate(&mut OsRng);
        let next_key = Base64UrlUnpadded::encode_string(next_signing_key.as_bytes());
        next_keys.insert(id.to_string(), next_key);
        Ok(())
    }

    // Rotate keys
    // pub fn rotate(&self) -> anyhow::Result<()> {
    //     let mut keys = self.keys.lock().map_err(|_| {
    //         anyhow!("failed to lock keys mutex")
    //     })?;
    //     let mut next_keys = self.next_keys.lock().map_err(|_| {
    //         anyhow!("failed to lock next keys mutex")
    //     })?;

    //     for (id, next_key) in next_keys.iter() {
    //         *keys.entry(id.clone()).or_insert(next_key.clone()) = next_key.clone();
    //     }
    //     next_keys.clear();
    //     for id in keys.keys() {
    //         let signing_key = SigningKey::generate(&mut OsRng);
    //         let key = Base64UrlUnpadded::encode_string(signing_key.as_bytes());
    //         next_keys.insert(id.clone(), key);
    //     }
    //     Ok(())
    // }

    // Get a public JWK for a key in the keyring.
    //
    // This will always return a result if it can. If the key is not found, one
    // will be generated with the specified ID.
    pub fn jwk(&self, id: impl ToString + Clone) -> anyhow::Result<PublicKeyJwk> {
        let keys = self.keys.lock().map_err(|_| {
            anyhow!("failed to lock keys mutex")
        })?;
        let secret = match keys.get(&id.to_string()) {
            Some(secret) => secret,
            None => {
                self.add_key(id.clone())?;
                keys.get(&id.to_string()).ok_or_else(|| {
                    anyhow!("key not found after generating new key")
                })?
            }
        };
        let key_bytes = Base64UrlUnpadded::decode_vec(&secret)?;
        let secret_key: ed25519_dalek::SecretKey =
            key_bytes.try_into().map_err(|_| anyhow::anyhow!("invalid secret key"))?;
        let signing_key = SigningKey::from_bytes(&secret_key);
        let verifying_key = signing_key.verifying_key().as_bytes().to_vec();
        Ok(PublicKeyJwk::from_bytes(&verifying_key)?)
    }

    // Get a public multibase key for a key in the keyring.
    // pub fn multibase(&self, id: impl ToString + Clone) -> anyhow::Result<String> {
    //     let key = self.jwk(id)?;
    //     Ok(key.to_multibase()?)
    // }

    // Get a public JWK for a next key in the keyring.
    //
    // This will fail with an error if the key is not found or any encoding
    // errors occur.
    // pub fn next_jwk(&self, id: impl ToString + Clone) -> anyhow::Result<PublicKeyJwk> {
    //     let keys = self.next_keys.lock().map_err(|_| {
    //         anyhow!("failed to lock keys mutex")
    //     })?;
    //     if let Some(secret) = keys.get(&id.to_string()).cloned() {
    //         let key_bytes = Base64UrlUnpadded::decode_vec(&secret)?;
    //         let secret_key: ed25519_dalek::SecretKey =
    //             key_bytes.try_into().map_err(|_| anyhow::anyhow!("invalid secret key"))?;
    //         let signing_key = SigningKey::from_bytes(&secret_key);
    //         let verifying_key = signing_key.verifying_key().as_bytes().to_vec();
    //         return Ok(PublicKeyJwk::from_bytes(&verifying_key)?);
    //     }
    //     Err(anyhow!("key not found"))
    // }

    // Get a public multibase key for a next key in the keyring.
    //
    // Will fail with an error if the key is not found or any encoding errors
    // occur.
    // pub fn next_multibase(&self, id: impl ToString + Clone) -> anyhow::Result<String> {
    //     let key = self.next_jwk(id)?;
    //     Ok(key.to_multibase()?)
    // }

    // Get a `did:key` for the specified key in the keyring.
    //
    // This will always return a result if it can. If the key is not found, one
    // will be generated with the specified ID.
    // 
    // Will fail if any encoding errors occur.
    // pub fn did_key(&self, id: impl ToString + Clone) -> anyhow::Result<String> {
    //     let key = self.multibase(id)?;
    //     Ok(format!("did:key:{key}#{key}"))
    // }
}

impl Signer for Keyring {
    async fn try_sign(&self, msg: &[u8]) -> anyhow::Result<Vec<u8>> {
        let keys = self.next_keys.lock().map_err(|_| {
            anyhow!("failed to lock keys mutex")
        })?;
        if let Some(secret) = keys.get("signing").cloned() {
            let key_bytes = Base64UrlUnpadded::decode_vec(&secret)?;
            let secret_key: ed25519_dalek::SecretKey =
                key_bytes.try_into().map_err(|_| anyhow::anyhow!("invalid secret key"))?;
            let signing_key = SigningKey::from_bytes(&secret_key);
            return Ok(signing_key.sign(msg).to_bytes().to_vec());
        }
        Err(anyhow!("key not found"))
    }

    async fn verifying_key(&self) -> anyhow::Result<Vec<u8>> {
        let keys = self.next_keys.lock().map_err(|_| {
            anyhow!("failed to lock keys mutex")
        })?;
        if let Some(secret) = keys.get("signing").cloned() {
            let key_bytes = Base64UrlUnpadded::decode_vec(&secret)?;
            let secret_key: ed25519_dalek::SecretKey =
                key_bytes.try_into().map_err(|_| anyhow::anyhow!("invalid secret key"))?;
            let signing_key = SigningKey::from_bytes(&secret_key);
            let verifying_key = signing_key.verifying_key().as_bytes().to_vec();
            return Ok(verifying_key);
        }
        Err(anyhow!("key not found"))
    }

    fn algorithm(&self) -> Algorithm {
        Algorithm::EdDSA
    }

    async fn verification_method(&self) -> anyhow::Result<String> {
        let vm = self.verification_method.lock().unwrap();
        if vm.is_empty() {
            return Err(anyhow!("verification method not set"));
        }
        Ok(vm.clone())
    }
}
