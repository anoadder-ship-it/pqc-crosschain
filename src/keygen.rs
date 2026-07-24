use pqcrypto_rs::kem::kyber512;
use pqcrypto_rs::sign::dilithium_a2;
use zeroize::{Zeroize, Zeroizing};
use getrandom::getrandom;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct PqcKeypairs {
    pub kyber_pk: Vec<u8>,
    kyber_sk: Zeroizing<Vec<u8>>,
    pub dilithium_pk: Vec<u8>,
    dilithium_sk: Zeroizing<Vec<u8>>,
    lamport_seeds: [u8; 96],
}

impl PqcKeypairs {
    pub fn generate() -> Self {
        let kyber = kyber512::keypair();
        let dilithium = dilithium_a2::keypair();
        
        let mut seeds = [0u8; 96];
        getrandom(&mut seeds).expect("CSPRNG failure");

        Self {
            kyber_pk: kyber.pk,
            kyber_sk: Zeroizing::new(kyber.sk),
            dilithium_pk: dilithium.pk,
            dilithium_sk: Zeroizing::new(dilithium.sk),
            lamport_seeds: seeds,
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

    pub fn zeroize(&mut self) {
        self.kyber_sk.zeroize();
        self.dilithium_sk.zeroize();
        self.lamport_seeds.fill(0);
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