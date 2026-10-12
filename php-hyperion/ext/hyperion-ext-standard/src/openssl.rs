use hyperion_core::memory::nan_box::Value;
use hyperion_core::php_function;
use rand::{RngCore, rngs::OsRng};
use aes::Aes128;
use aes::Aes256;
use cbc::{Encryptor, Decryptor};
use cipher::{BlockEncryptMut, BlockDecryptMut, KeyIvInit};
use base64::{Engine as _, engine::general_purpose::STANDARD};

const OPENSSL_RAW_DATA: i32 = 1;

type Aes128CbcEnc = Encryptor<Aes128>;
type Aes128CbcDec = Decryptor<Aes128>;
type Aes256CbcEnc = Encryptor<Aes256>;
type Aes256CbcDec = Decryptor<Aes256>;

fn format_bytes_to_string(bytes: Vec<u8>, options: i32) -> String {
    if options & OPENSSL_RAW_DATA != 0 {
        unsafe { String::from_utf8_unchecked(bytes) }
    } else {
        STANDARD.encode(bytes)
    }
}

fn parse_string_to_bytes(data: &str, options: i32) -> Result<Vec<u8>, String> {
    if options & OPENSSL_RAW_DATA != 0 {
        Ok(data.as_bytes().to_vec())
    } else {
        STANDARD.decode(data.as_bytes()).map_err(|_| "Failed to base64 decode input".to_string())
    }
}

php_function! {
    native_openssl_cipher_iv_length(cipher_algo: String) {
        let algo = match cipher_algo {
            Some(a) => a.to_lowercase(),
            None => return Err("openssl_cipher_iv_length() expects exactly 1 parameter".to_string()),
        };
        
        let len = match algo.as_str() {
            "aes-128-cbc" | "aes-256-cbc" => 16,
            "aes-128-gcm" | "aes-256-gcm" => 12, // Laravel uses 12 bytes IV for GCM
            _ => return Ok(Value::new_bool(false)),
        };
        
        Ok(Value::new_int(len))
    }
}

php_function! {
    native_openssl_random_pseudo_bytes(length: Value) {
        if let Some(length_val) = length {
            let len = match length_val.deref().as_int() {
                Some(l) if l > 0 => (l as usize).min(50_000_000),
                _ => return Ok(Value::new_bool(false)),
            };
            
            let mut bytes = vec![0u8; len];
            OsRng.fill_bytes(&mut bytes);
            
            let s = unsafe { String::from_utf8_unchecked(bytes) };
            let boxed = crate::into_raw(Box::new(s));
            Ok(Value::new_string_ptr(boxed as *mut ()))
        } else {
            Err("openssl_random_pseudo_bytes() expects exactly 1 parameter".to_string())
        }
    }
}

php_function! {
    native_openssl_encrypt(data: String, cipher_algo: String, passphrase: String, options: Value, iv: String) {
        let data = match data {
            Some(d) => d,
            None => return Err("openssl_encrypt() missing data".to_string()),
        };
        let algo = match cipher_algo {
            Some(a) => a.to_lowercase(),
            None => return Err("openssl_encrypt() missing algo".to_string()),
        };
        let key_str = match passphrase {
            Some(p) => p,
            None => return Err("openssl_encrypt() missing key".to_string()),
        };
        let opts = options.and_then(|v| v.as_int()).unwrap_or(0);
        let iv_str = iv.map(|s| s.as_str()).unwrap_or("");
        
        let key = key_str.as_bytes();
        let iv_bytes = iv_str.as_bytes();
        
        let result = match algo.as_str() {
            "aes-128-cbc" => {
                if key.len() < 16 || iv_bytes.len() < 16 { return Ok(Value::new_bool(false)); }
                let enc = Aes128CbcEnc::new_from_slices(&key[..16], &iv_bytes[..16]).map_err(|e| e.to_string())?;
                let mut buf = data.as_bytes().to_vec();
                let pt_len = buf.len();
                let block_size = 16;
                let pad_len = block_size - (pt_len % block_size);
                buf.resize(pt_len + pad_len, 0);
                let ct = enc.encrypt_padded_mut::<cipher::block_padding::Pkcs7>(&mut buf, pt_len).map_err(|e| e.to_string())?;
                format_bytes_to_string(ct.to_vec(), opts)
            },
            "aes-256-cbc" => {
                if key.len() < 32 || iv_bytes.len() < 16 { return Ok(Value::new_bool(false)); }
                let enc = Aes256CbcEnc::new_from_slices(&key[..32], &iv_bytes[..16]).map_err(|e| e.to_string())?;
                let mut buf = data.as_bytes().to_vec();
                let pt_len = buf.len();
                let block_size = 16;
                let pad_len = block_size - (pt_len % block_size);
                buf.resize(pt_len + pad_len, 0);
                let ct = enc.encrypt_padded_mut::<cipher::block_padding::Pkcs7>(&mut buf, pt_len).map_err(|e| e.to_string())?;
                format_bytes_to_string(ct.to_vec(), opts)
            },
            _ => return Ok(Value::new_bool(false)),
        };
        
        let boxed = crate::into_raw(Box::new(result));
        Ok(Value::new_string_ptr(boxed as *mut ()))
    }
}

php_function! {
    native_openssl_decrypt(data: String, cipher_algo: String, passphrase: String, options: Value, iv: String) {
        let data_str = match data {
            Some(d) => d,
            None => return Err("openssl_decrypt() missing data".to_string()),
        };
        let algo = match cipher_algo {
            Some(a) => a.to_lowercase(),
            None => return Err("openssl_decrypt() missing algo".to_string()),
        };
        let key_str = match passphrase {
            Some(p) => p,
            None => return Err("openssl_decrypt() missing key".to_string()),
        };
        let opts = options.and_then(|v| v.as_int()).unwrap_or(0);
        let iv_str = iv.map(|s| s.as_str()).unwrap_or("");
        
        let key = key_str.as_bytes();
        let iv_bytes = iv_str.as_bytes();
        
        let ct = match parse_string_to_bytes(data_str, opts) {
            Ok(b) => b,
            Err(_) => return Ok(Value::new_bool(false)),
        };
        
        let result = match algo.as_str() {
            "aes-128-cbc" => {
                if key.len() < 16 || iv_bytes.len() < 16 { return Ok(Value::new_bool(false)); }
                let dec = Aes128CbcDec::new_from_slices(&key[..16], &iv_bytes[..16]).map_err(|e| e.to_string())?;
                let mut buf = ct.clone();
                let pt = match dec.decrypt_padded_mut::<cipher::block_padding::Pkcs7>(&mut buf) {
                    Ok(p) => p,
                    Err(_) => return Ok(Value::new_bool(false)),
                };
                unsafe { String::from_utf8_unchecked(pt.to_vec()) }
            },
            "aes-256-cbc" => {
                if key.len() < 32 || iv_bytes.len() < 16 { return Ok(Value::new_bool(false)); }
                let dec = Aes256CbcDec::new_from_slices(&key[..32], &iv_bytes[..16]).map_err(|e| e.to_string())?;
                let mut buf = ct.clone();
                let pt = match dec.decrypt_padded_mut::<cipher::block_padding::Pkcs7>(&mut buf) {
                    Ok(p) => p,
                    Err(_) => return Ok(Value::new_bool(false)),
                };
                unsafe { String::from_utf8_unchecked(pt.to_vec()) }
            },
            _ => return Ok(Value::new_bool(false)),
        };
        
        let boxed = crate::into_raw(Box::new(result));
        Ok(Value::new_string_ptr(boxed as *mut ()))
    }
}
