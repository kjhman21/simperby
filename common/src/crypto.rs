//! A set of types and functions related to cryptography, that are widely used in the entire Simperby project.
use ed25519::signature::{Signer, Verifier};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize, Clone)]
pub enum CryptoError {
    /// When the data format is not valid.
    #[error("invalid format: {0}")]
    InvalidFormat(String),
    #[error("verification failed")]
    VerificationFailed,
}

type Error = CryptoError;

/// A cryptographic hash.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize, Hash)]
pub struct Hash256 {
    pub hash: [u8; 32],
}

impl Hash256 {
    pub const fn zero() -> Self {
        Hash256 { hash: [0; 32] }
    }

    /// Hashes the given data.
    pub fn hash(data: impl AsRef<[u8]>) -> Self {
        Hash256 {
            hash: *blake3::hash(data.as_ref()).as_bytes(),
        }
    }

    pub fn from_array(data: [u8; 32]) -> Self {
        Hash256 {
            hash: *blake3::Hash::from(data).as_bytes(),
        }
    }

    pub fn aggregate(&self, other: &Self) -> Self {
        Self::hash([self.hash, other.hash].concat())
    }
}

impl std::convert::AsRef<[u8]> for Hash256 {
    fn as_ref(&self) -> &[u8] {
        &self.hash
    }
}

impl fmt::Display for Hash256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "?")
    }
}

/// A cryptographic signature.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct Signature {
    signature: Vec<u8>,
}

impl Signature {
    /// Creates a new signature from the given data and keys.
    pub fn sign(
        data: Hash256,
        public_key: &PublicKey,
        private_key: &PrivateKey,
    ) -> Result<Self, Error> {
        check_keypair_match(public_key, private_key)?;
        let public_key = ed25519_dalek::PublicKey::from_bytes(&public_key.key)
            .map_err(|_| Error::InvalidFormat(format!("public key: {}", public_key)))?;
        let private_key = ed25519_dalek::SecretKey::from_bytes(&private_key.key)
            .map_err(|_| Error::InvalidFormat("private key: [omitted]".to_owned()))?;
        let keypair = ed25519_dalek::Keypair {
            secret: private_key,
            public: public_key,
        };
        Ok(Signature {
            signature: keypair.sign(data.hash.as_ref()).to_bytes().to_vec(),
        })
    }

    /// Verifies the signature against the given data and public key.
    pub fn verify(&self, data: Hash256, public_key: &PublicKey) -> Result<(), Error> {
        let signature = ed25519_dalek::Signature::from_bytes(&self.signature)
            .map_err(|_| Error::InvalidFormat(format!("signature: {}", self)))?;
        let public_key = ed25519_dalek::PublicKey::from_bytes(&public_key.key)
            .map_err(|_| Error::InvalidFormat(format!("public_key: {}", public_key)))?;
        public_key
            .verify(data.as_ref(), &signature)
            .map_err(|_| Error::VerificationFailed)
    }
}

/// A signature that is explicitly marked with the type of the signed data.
///
/// This implies that the signature is created on `Hash256::hash(serde_json::to_vec(T).unwrap())`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct TypedSignature<T> {
    pub signature: Signature,
    pub _mark: std::marker::PhantomData<T>,
}

impl<T: serde::Serialize> TypedSignature<T> {
    /// Creates a new signature from the given data and keys.
    pub fn sign(data: &T, public_key: &PublicKey, private_key: &PrivateKey) -> Result<Self, Error> {
        let data = serde_json::to_vec(data).map_err(|_| Error::InvalidFormat("data".to_owned()))?;
        let data = Hash256::hash(data);
        Signature::sign(data, public_key, private_key).map(|signature| TypedSignature {
            signature,
            _mark: std::marker::PhantomData,
        })
    }

    pub fn new(signature: Signature) -> Self {
        TypedSignature {
            signature,
            _mark: std::marker::PhantomData,
        }
    }

    /// Verifies the signature against the given data and public key.
    pub fn verify(&self, data: &T, public_key: &PublicKey) -> Result<(), Error> {
        let data = serde_json::to_vec(data).map_err(|_| Error::InvalidFormat("data".to_owned()))?;
        let data = Hash256::hash(data);
        self.signature.verify(data, public_key)
    }
}

impl std::convert::AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.signature
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "?")
    }
}

/// A public key.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct PublicKey {
    key: Vec<u8>,
}

impl std::convert::AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.key
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "?")
    }
}

/// A private key.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct PrivateKey {
    pub key: Vec<u8>,
}

impl std::convert::AsRef<[u8]> for PrivateKey {
    fn as_ref(&self) -> &[u8] {
        &self.key
    }
}

/// Checkes whether the given public and private keys match.
pub fn check_keypair_match(public_key: &PublicKey, private_key: &PrivateKey) -> Result<(), Error> {
    let msg = "Some Random Message".as_bytes();
    let signature = Signature::sign(Hash256::hash(msg), public_key, private_key)?;
    signature.verify(Hash256::hash(msg), public_key)
}

/// Generates a new keypair using the seed.
pub fn generate_keypair(seed: impl AsRef<[u8]>) -> (PublicKey, PrivateKey) {
    let mut seed_: [u8; 32] = [0; 32];
    for (i, x) in Hash256::hash(seed).as_ref()[0..32].iter().enumerate() {
        seed_[i] = *x;
    }
    let mut rng = rand::rngs::StdRng::from_seed(seed_);
    let signing_key = ed25519_dalek::Keypair::generate(&mut rng);
    (
        PublicKey {
            key: signing_key.public.to_bytes().to_vec(),
        },
        PrivateKey {
            key: signing_key.secret.to_bytes().to_vec(),
        },
    )
}
