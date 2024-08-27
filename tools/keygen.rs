use rand::rngs::OsRng;
use rand::RngCore;
use secp256k1::{Secp256k1, SecretKey};

fn main() {
    let secp = Secp256k1::new();
    let mut rng = OsRng::default();
    let mut secret_key_bytes = [0u8; 32];
    rng.fill_bytes(&mut secret_key_bytes);
    let secret_key = SecretKey::from_slice(&secret_key_bytes).expect("32 bytes, within curve order");
    let secret_key_hex = hex::encode(secret_key.as_ref());
    println!("Secret Key: {}", secret_key_hex);
}
