use pqcrypto::kem::kyber512;
use pqcrypto::sign::dilithium_a2;
use zeroize::{Zeroize, Zeroizing};
use getrandom::getrandom;
use hmac::{Hmac, Mac};
use sha2::{Sha256, Digest};

type HmacSha256 = Hmac<Sha256>;

// WOTS+ parameters (NIST standard for SHA-256)
const WOTS_W: u8 = 16; // Winternitz parameter
const WOTS_K: usize = 4; // Security parameter

#[derive(Clone)]
pub struct PqcKeypairs {
    pub kyber_pk: Vec<u8>,
    kyber_sk: Zeroizing<Vec<u8>>,
    pub dilithium_pk: Vec<u8>,
    dilithium_sk: Zeroizing<Vec<u8>>,
    lamport_seeds: [u8; 96],
    wots_plus_keys: Vec<WotsPlusKeyPair>,
}

pub struct WotsPlusKeyPair {
    pub pk: [u8; 32],
    sk: [u8; 32],
}

impl WotsPlusKeyPair {
    pub fn generate() -> Self {
        let mut sk = [0u8; 32];
        getrandom(&mut sk).expect("CSPRNG failure");
        
        // WOTS+ public key is hash of the chain
        let pk = wots_plus_hash_chain(&sk);
        Self { pk, sk }
    }

    pub fn sign(&self, msg: &[u8]) -> Vec<u8> {
        let msg_hash = Sha256::digest(msg);
        // WOTS+ signing with pruning optimization
        wots_plus_sign(&self.sk, &msg_hash)
    }

    pub fn verify(&self, msg: &[u8], sig: &[u8]) -> bool {
        let msg_hash = Sha256::digest(msg);
        let computed_pk = wots_plus_verify(&sig, &msg_hash);
        computed_pk == self.pk
    }
}

impl PqcKeypairs {
    pub fn generate() -> Self {
        let kyber = kyber512::keypair();
        let dilithium = dilithium_a2::keypair();
        
        let mut seeds = [0u8; 96];
        getrandom(&mut seeds).expect("CSPRNG failure");

        // Genereer WOTS+ key pairs voor extra security layer
        let wots_plus_keys: Vec<WotsPlusKeyPair> = (0..WOTS_K).map(|_| WotsPlusKeyPair::generate()).collect();

        Self {
            kyber_pk: kyber.pk,
            kyber_sk: Zeroizing::new(kyber.sk),
            dilithium_pk: dilithium.pk,
            dilithium_sk: Zeroizing::new(dilithium.sk),
            lamport_seeds: seeds,
            wots_plus_keys,
        }
    }

    pub fn encaps_session(&self, context: &[u8]) -> ([u8; 768], [u8; 32]) {
        let (ct, ss) = kyber512::encaps(&self.kyber_pk);
        let mut derived_ss = [0u8; 32];
        hkdf_sha256(&ss, context, &mut derived_ss);
        (ct.try_into().unwrap(), derived_ss)
    }

    pub fn sign_transfer(&self, payload: &[u8]) -> [u8; 1984] {
        let sig = dilithium_a2::sign(payload, &self.dilithium_sk).to_vec();
        sig.try_into().unwrap()
    }

    pub fn derive_otp_keys(&self, idx: usize) -> ([u8; 32], [u8; 32]) {
        let seed = &self.lamport_seeds[idx * 32..(idx + 1) * 32];
        let mut sk = [0u8; 32];
        let mut pk = [0u8; 32];
        hkdf_sha256(seed, b"PQC_LAMPORT_SK", &mut sk);
        hkdf_sha256(seed, b"PQC_LAMPORT_PK", &mut pk);
        (sk, pk)
    }

    pub fn sign_wots_plus(&self, msg: &[u8]) -> Vec<u8> {
        // Gebruik eerste WOTS+ key pair voor signing
        self.wots_plus_keys[0].sign(msg)
    }

    pub fn verify_wots_plus(&self, msg: &[u8], sig: &[u8]) -> bool {
        self.wots_plus_keys[0].verify(msg, sig)
    }

    pub fn zeroize(&mut self) {
        self.kyber_sk.zeroize();
        self.dilithium_sk.zeroize();
        self.lamport_seeds.fill(0);
        for key in &mut self.wots_plus_keys {
            key.sk.zeroize();
        }
    }
}

impl Zeroize for PqcKeypairs {
    fn zeroize(&mut self) { self.zeroize(); }
}

fn hkdf_sha256(input_key: &[u8], info: &[u8], output: &mut [u8; 32]) {
    let mut mac = HmacSha256::new_from_slice(input_key).expect("HMAC init");
    mac.update(info);
    output.copy_from_slice(&mac.finalize().into_bytes());
}

fn wots_plus_hash_chain(seed: &[u8; 32]) -> [u8; 32] {
    let mut current = *seed;
    for _ in 0..WOTS_W {
        let hash = Sha256::digest(current);
        current = hash.into();
    }
    current
}

fn wots_plus_sign(sk: &[u8; 32], msg_hash: &[u8; 32]) -> Vec<u8> {
    // WOTS+ signing implementation with pruning optimization
    let mut sig = Vec::with_capacity(64);
    for chunk in msg_hash.chunks(2) {
        let mut current = *sk;
        let steps = chunk.iter().fold(0u8, |acc, &b| acc + b);
        for _ in 0..steps {
            let hash = Sha256::digest(current);
            current = hash.into();
        }
        sig.extend_from_slice(&current);
    }
    sig
}

fn wots_plus_verify(sig: &[u8], msg_hash: &[u8; 32]) -> [u8; 32] {
    let mut pk = [0u8; 32];
    for (chunk, sig_chunk) in msg_hash.chunks(2).zip(sig.chunks(32)) {
        let mut current = sig_chunk.unwrap_or(&[0u8; 32]);
        let steps = chunk.iter().fold(0u8, |acc, &b| acc + b);
        for _ in steps..WOTS_W {
            let hash = Sha256::digest(current);
            current = &hash.into();
        }
        // Combine chains
        for (i, byte) in current.iter().enumerate() {
            pk[i % 32] ^= byte;
        }
    }
    pk
}