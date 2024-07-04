use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;

use pkcs8::DecodePrivateKey;
use rustls::pki_types::PrivateKeyDer;
use rustls::sign::{Signer, SigningKey};
use rustls::{SignatureAlgorithm, SignatureScheme};
use signature::{RandomizedSigner, SignatureEncoding};
use wolfcrypt_rs::*;
use foreign_types::{ForeignType, ForeignTypeRef, Opaque};
use std::ptr::NonNull;
use std::mem;

pub struct ECCKeyObjectRef(Opaque);
unsafe impl ForeignTypeRef for ECCKeyObjectRef {
    type CType = ecc_key;
}

pub struct ECCKeyObject(NonNull<ecc_key>);
unsafe impl Sync for ECCKeyObject{}
unsafe impl Send for ECCKeyObject{}
unsafe impl ForeignType for ECCKeyObject {
    type CType = ecc_key;

    type Ref = ECCKeyObjectRef;

    unsafe fn from_ptr(ptr: *mut Self::CType) -> Self {
        Self(NonNull::new_unchecked(ptr))
    }

    fn as_ptr(&self) -> *mut Self::CType {
        self.0.as_ptr()
    }
}


#[derive(Clone, Debug)]
pub struct EcdsaSigningKeyP256 {
    key: Arc<p256::ecdsa::SigningKey>,
    scheme: SignatureScheme,
}

impl TryFrom<PrivateKeyDer<'_>> for EcdsaSigningKeyP256 {
    type Error = pkcs8::Error;

    fn try_from(value: PrivateKeyDer<'_>) -> Result<Self, Self::Error> {
        match value {
            PrivateKeyDer::Pkcs8(der) => {
                unsafe {
                    let mut ecc_key_struct: ecc_key = mem::zeroed();
                    let ecc_key_object = ECCKeyObject::from_ptr(&mut ecc_key_struct);
                    let mut der_size: i32;
                    let curve_oid: u8 = mem::zeroed();
                    let curve_oid_sz: u32 = 0;
                    let pkcs8: u8 = mem::zeroed();
                    let pkcs8_sz: u32 = 0;
                    let mut ret;

                    der_size = wc_EccKeyDerSize(ecc_key_object.as_ptr(), 1);
                    if der_size <= 0 {
                        panic!("error while calling wc_EccKeyDerSize");
                    }
                    der_size = wc_EccKeyToDer(ecc_key_object.as_ptr(), 
                                             der.secret_pkcs8_der().as_ptr() as *mut u8, 
                                             der_size as u32);
                    if der_size <= 0 {
                        panic!("error while calling wc_EccKeyDerSize");
                    }

                    let dp_ptr = ecc_key_struct.dp;
                    let dp_struct = &*dp_ptr;
                    let oid_sum = dp_struct.oidSum;

                    ret = wc_ecc_get_oid(oid_sum, curve_oid as *mut *const u8, curve_oid_sz as *mut u32);
                    if ret != 0 {
                        panic!("error while calling wc_ecc_get_oid");
                    }

                    let null_value: *mut u8 = mem::zeroed();

                    ret = wc_CreatePKCS8Key(null_value, 
                        pkcs8_sz as *mut u32, 
                        der.secret_pkcs8_der().as_ptr() as *mut u8, 
                        der_size as u32, 
                        Key_Sum_ECDSAk as i32,
                        curve_oid as *const u8, 
                        curve_oid_sz); // get size needed in pkcs8_sz
                    if ret != 0 {
                        panic!("error while calling wc_CreatePKCS8Key");
                    }

                    ret = wc_CreatePKCS8Key(pkcs8 as *mut u8, 
                        pkcs8_sz as *mut u32, 
                        der.secret_pkcs8_der().as_ptr() as *mut u8,
                        der_size as u32, 
                        Key_Sum_ECDSAk as i32,
                        curve_oid as *const u8, 
                        curve_oid_sz);
                    if ret != 0 {
                        panic!("error while calling wc_CreatePKCS8Key");
                    }

                    // Leaving this here just for testing purposes.
                    p256::ecdsa::SigningKey::from_pkcs8_der(der.secret_pkcs8_der()).map(|kp| Self {
                        key: Arc::new(kp),
                        scheme: SignatureScheme::ECDSA_NISTP256_SHA256,
                    })
                }
            }
            _ => panic!("unsupported private key format"),
        }
    }
}

impl SigningKey for EcdsaSigningKeyP256 {
    fn choose_scheme(&self, offered: &[SignatureScheme]) -> Option<Box<dyn Signer>> {
        if offered.contains(&self.scheme) {
            Some(Box::new(self.clone()))
        } else {
            None
        }
    }

    fn algorithm(&self) -> SignatureAlgorithm {
        SignatureAlgorithm::ECDSA
    }
}

impl Signer for EcdsaSigningKeyP256 {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, rustls::Error> {
        self.key
            .try_sign_with_rng(&mut rand_core::OsRng, message)
            .map_err(|_| rustls::Error::General("signing failed".into()))
            .map(|sig: p256::ecdsa::DerSignature| sig.to_vec())
    }

    fn scheme(&self) -> SignatureScheme {
        self.scheme
    }
}
