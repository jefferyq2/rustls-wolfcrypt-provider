use alloc::boxed::Box;

use rustls::crypto::hash;
use sha2::Digest;
use std::mem;
use wolfcrypt_rs::{wc_Sha256, word32, wc_InitSha256, wc_Sha256Update, wc_Sha256Final};

pub struct WCSha256;

impl hash::Hash for WCSha256 {
    fn start(&self) -> Box<dyn hash::Context> {
        unsafe {
            let sha256_struct: wc_Sha256 = mem::zeroed();
            let hash: [u8; 32] = [0; 32];

            let hasher = WCHasher {
                sha256_struct: sha256_struct,
                hash: hash
            };
            Box::new(WCSha256Context(hasher))
        }
    }

    fn hash(&self, data: &[u8]) -> hash::Output {
        let mut hasher = self.start();
        hasher.update(data);
        hasher.finish()
    }

    fn algorithm(&self) -> hash::HashAlgorithm {
        hash::HashAlgorithm::SHA256
    }

    fn output_len(&self) -> usize {
        32
    }
}

struct WCHasher {
    sha256_struct: wc_Sha256,
    hash: [u8; 32],
}

impl WCHasher {
    fn wchasher_init(&mut self) {
        unsafe {
            let ret;

            ret = wc_InitSha256(&mut self.sha256_struct);
            if ret != 0 {
                panic!("wc_InitSha256 failed with ret: {}", ret);
            }
        }
    }

    fn wchasher_update(&mut self, data: &[u8]) {
        unsafe {
            let ret;
            let length: word32 = data.len() as word32;

            ret = wc_Sha256Update(&mut self.sha256_struct, data.as_ptr() as *const u8, length);
            if ret != 0 {
                panic!("wc_Sha256Update failed with ret: {}", ret);
            }
        }
    }

    fn wchasher_final(&mut self) -> &[u8] {
        unsafe {
            let ret;

            ret = wc_Sha256Final(&mut self.sha256_struct, self.hash.as_mut_ptr());
            if ret != 0 {
                panic!("wc_Sha256Final failed with ret: {}", ret);
            }

            &self.hash
        }
    }
}

unsafe impl Sync for WCHasher{}
unsafe impl Send for WCHasher{}
impl Clone for WCHasher {
    fn clone(&self) -> WCHasher {
        WCHasher {
            sha256_struct: self.sha256_struct.clone(),
            hash: self.hash.clone()
        }
    }
}

struct WCSha256Context(WCHasher);

impl hash::Context for WCSha256Context {
    fn fork_finish(&self) -> hash::Output {
        hash::Output::new(&self.0.clone().wchasher_final()[..])
    }

    fn fork(&self) -> Box<dyn hash::Context> {
        Box::new(WCSha256Context(self.0.clone()))
    }

    fn finish(mut self: Box<Self>) -> hash::Output {
        hash::Output::new(&self.0.wchasher_final()[..])
    }

    fn update(&mut self, data: &[u8]) {
        self.0.wchasher_update(data);
    }
}

#[cfg(test)]
mod tests {
    use super::WCSha256;
    use rustls::crypto::hash::Hash;
    use hex_literal::hex;
    use std::println;

    #[test]
    fn sha256_test() {
        unsafe {
            let WCSha256_struct = WCSha256;
            let hash1 = WCSha256_struct.hash("hello".as_bytes());
            let hash2 = WCSha256_struct.hash("hello".as_bytes());

            let hash_str1 = hex::encode(hash1);
            let hash_str2 = hex::encode(hash2);


            assert_eq!(
                hash_str1,
                hash_str2
            );
        }
    }
}
