/*
 * This file is part of Monero Builders' library libmonero
 *
 * Copyright (c) 2023-2024, Monero Builders (monero.builders)
 * All Rights Reserved
 * The code is distributed under MIT license, see LICENSE file for details.
 * Generated by Monero Builders
 *
 */

//! # Keys
//!
//! This module is for everything related to keys, such as generating seeds, deriving keys from seeds, deriving public keys from private keys, and deriving addresses from public keys etc.

use crate::crypt::ed25519::sc_reduce32;
use crate::mnemonics::original::wordsets::{WordsetOriginal, WORDSETSORIGINAL};
use crc32fast::Hasher;
use curve25519_dalek::{constants::ED25519_BASEPOINT_TABLE, EdwardsPoint, Scalar};
use rand::Rng;
use sha3::{Digest, Keccak256};
use core::panic;
use std::ops::Mul;
use std::vec;

/// Returns cryptographically secure random element of the given array
fn secure_random_element<'x>(array: &'x [&'x str]) -> &'x str {
    let mut rng = rand::thread_rng();
    let random_index = rng.gen_range(0..array.len());
    array[random_index]
}

// Returns cryptographically secure random bits of given length
fn get_random_bits(length: u64) -> Vec<bool> {
    let mut rng = rand::thread_rng();
    let mut bit_array = Vec::new();
    for _ in 0..length {
        bit_array.push(rng.gen_bool(0.5));
    }
    bit_array
}

/// Calculates CRC32 checksum index for given array (probably the seed)
fn get_checksum_index(array: &[&str], prefix_length: usize) -> usize {
    let mut trimmed_words: String = String::new();
    for word in array {
        trimmed_words.push_str(&word[0..prefix_length]);
    }
    let mut hasher = Hasher::new();
    hasher.update(trimmed_words.as_bytes());
    usize::try_from(hasher.finalize()).unwrap() % array.len()
}

/// Generates a cryptographically secure 1626-type (25-word) seed for given language
fn generate_original_seed(language: &str) -> Vec<&str> {
    // Check if language is supported
    if !WORDSETSORIGINAL.iter().any(|x| x.name == language) {
        panic!("Language not found");
    }
    // Generate seed
    let mut seed: Vec<&str> = Vec::new();
    let mut prefix_len: usize = 3;
    for wordset in WORDSETSORIGINAL.iter() {
        if wordset.name == language {
            prefix_len = wordset.prefix_len;
            for _ in 0..24 {
                let word = secure_random_element(&wordset.words[..]);
                seed.push(word);
            }
            break;
        } else {
            continue;
        }
    }
    // Add checksum word
    let checksum_index = get_checksum_index(&seed, prefix_len);
    seed.push(seed[checksum_index]);
    // Finally, return the seed
    seed
}

/// Generates a cryptographically secure 1626-type (13-word) seed for given language
fn generate_mymonero_seed(language: &str) -> Vec<&str> {
    // Check if language is supported
    if !WORDSETSORIGINAL.iter().any(|x| x.name == language) {
        panic!("Language not found");
    }
    // Generate seed
    let mut seed: Vec<&str> = Vec::new();
    let mut prefix_len: usize = 3;
    for wordset in WORDSETSORIGINAL.iter() {
        if wordset.name == language {
            prefix_len = wordset.prefix_len;
            for _ in 0..12 {
                let word = secure_random_element(&wordset.words[..]);
                seed.push(word);
            }
            break;
        } else {
            continue;
        }
    }
    // Add checksum word
    let checksum_index = get_checksum_index(&seed, prefix_len);
    seed.push(seed[checksum_index]);
    // Finally, return the seed
    seed
}

fn print_seed_pretty(seed: Vec<Vec<bool>>) {
    for word in seed.iter() {
        for bit in word.iter() {
            print!("{}", if *bit { "1" } else { "0" });
        }
        println!();
    }
}

static POLYSEED_MUL2_TABLE: [u16; 8] = [5, 7, 1, 3, 13, 15, 9, 11];

fn gf_elem_mul2(x: u16) -> u16 {
    if x < 1024 {
        return 2 * x;
    }
    POLYSEED_MUL2_TABLE[x as usize % 8] + 16 * ((x - 1024) / 8)
}

fn gf_poly_eval(coeff: &[u16; 16]) -> u16 {
    // Horner's method at x = 2
    let mut result = coeff[15];
    for i in (0..15).rev() {
        result = gf_elem_mul2(result) ^ coeff[i];
    }
    result
}

/*
/// Generates a cryptographically secure 2048-type (16-word) seed for given language
fn generate_polyseed_seed(language: &str) -> Vec<&str> {
    // Encoding

    // Each word contains 11 bits of information. The data are encoded as follows:
    // word # 	contents
    // 1 	checksum (11 bits)
    // 2-6 	secret seed (10 bits) + features (1 bit)
    // 7-16 	secret seed (10 bits) + birthday (1 bit)

    // In total, there are 11 bits for the checksum, 150 bits for the secret seed, 5 feature bits and 10 birthday bits. Because the feature and birthday bits are non-random, they are spread over the 15 data words so that two different mnemonic phrases are unlikely to have the same word in the same position.
    // Checksum
    // The mnemonic phrase can be treated as a polynomial over GF(2048), which enables the use of an efficient Reed-Solomon error correction code with one check word. All single-word errors can be detected and all single-word erasures can be corrected without false positives.
    
    // Check if language is supported
    if !WORDSETSPOLYSEED.iter().any(|x| x.name == language) {
        panic!("Language not found");
    }
    // Get birthday
    const POLYSEEDEPOCH: u64 = 1635768000; // The epoch for Polyseed birthdays. 1st November 2021 12:00 UTC
    const TIMESTEP: u64 = 2629746; // The time step for Polyseed. 1/12 of the Gregorian year
    let birthday: u16 = ((SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - POLYSEEDEPOCH)
        / TIMESTEP)
        .try_into()
        .unwrap(); // The birthday of the seed from how much approximate months have passed since the epoch
    let mut birthday_bits: Vec<bool> = birthday
        .to_be_bytes()
        .to_vec()
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |i| (byte >> i) & 1 == 1))
        .collect();
    birthday_bits.drain(..6);
    let seed_bits = get_random_bits(150); // Get 150 random bits
    let features_bits = [false; 5]; // We don't use any feature while generating the seed
    let mut words_bits: Vec<Vec<bool>> = Vec::with_capacity(15); // 16 minus 1 checksum word
    // Add secret seed and features bits
    for (index, item) in features_bits.iter().enumerate() {
        let mut word: Vec<bool> = Vec::with_capacity(11);
        let sss = index * 10;
        let sse = (index + 1) * 10;
        let ssi = seed_bits[sss..sse].to_vec();
        for bit in ssi {
            word.push(bit);
        }
        word.push(*item);
        words_bits.push(word);
    }
    // Add rest of the seed and birthday bits
    for i in 5..15 {
        let mut word: Vec<bool> = Vec::with_capacity(11);
        let sss = i * 10;
        let sse = (i + 1) * 10;
        let ssi = seed_bits[sss..sse].to_vec();
        for bit in ssi {
            word.push(bit);
        }
        word.push(birthday_bits[i - 5]);
        words_bits.push(word);
    }
    // Choose words based on each bits, corresponding to 0-2047
    let mut words_indexes: [u16; 16] = [0; 16];
    for (index, word_bits) in words_bits.iter().enumerate() {
        let mut word_index: u16 = 0;
        for (i, bit) in word_bits.iter().enumerate() {
            if *bit {
                word_index += 2u16.pow((10 - i) as u32);
            }
        }
        words_indexes[index] = word_index;
    }
    print_seed_pretty(words_bits);
    // Calculate checksum based on comment describing
    let checksum = gf_poly_eval(&words_indexes);
    // Add checksum word
    let mut seed: Vec<&str> = Vec::new();
    seed.push(WORDSETSPOLYSEED[0].words[checksum as usize]);
    // Add rest of the words
    for index in 0..15 {
        seed.push(WORDSETSPOLYSEED[0].words[words_indexes[index] as usize]);
    }
    // Finally, return the seed
    seed
}
*/

/// Generates a cryptographically secure mnemonic phrase for given language and seed type
///
/// Available seed types:
/// - `original` : (25-word)
///     - `en` (English)
///     - `eo` (Esperanto)
///     - `fr` (French)
///     - `it` (Italian)
///     - `jp` (Japanese) (Works but not recommended)
///     - `lj` (Lojban)
///     - `pt` (Portuguese)
///     - `ru` (Russian)
/// - `mymonero` : (13-word, MyMonero wallet type)
///     - `en`, `eo`, `fr`, `it`, `jp`, `lj`, `pt`, `ru` (same as original)
/// - `polyseed` : (TO BE IMPLEMENTED)
/// > DISCLAIMER: polyseed is not implemented yet
///
/// Example:
/// ```
/// use libmonero::keys::generate_seed;
///
/// let mnemonic: Vec<String> = generate_seed("en", "original");
/// // Not equal to the example below because the seed is generated randomly, but the seed is valid
/// assert_ne!(mnemonic, vec!["tissue", "raking", "haunted", "huts", "afraid", "volcano", "howls", "liar", "egotistic", "befit", "rounded", "older", "bluntly", "imbalance", "pivot", "exotic", "tuxedo", "amaze", "mostly", "lukewarm", "macro", "vocal", "hounded", "biplane", "rounded"].iter().map(|&s| s.to_string()).collect::<Vec<String>>());
/// ```
pub fn generate_seed(language: &str, seed_type: &str) -> Vec<String> {
    let seed = match seed_type {
        "original" => generate_original_seed(language),
        "mymonero" => generate_mymonero_seed(language),
        "polyseed" => panic!("Polyseed is not implemented yet"),
        _ => panic!("Invalid seed type"),
    };
    let mut seed_string: Vec<String> = Vec::new();
    for word in seed {
        seed_string.push(word.to_string());
    }
    seed_string
}

/// Swaps endianness of a 4-byte string
fn swap_endian_4_byte(s: &str) -> String {
    format!("{}{}{}{}", &s[6..8], &s[4..6], &s[2..4], &s[0..2])
}

/// Derives hexadecimal seed from the given mnemonic seed
///
/// Example:
/// ```
/// use libmonero::keys::derive_hex_seed;
///
/// let mnemonic: Vec<String> = vec!["tissue", "raking", "haunted", "huts", "afraid", "volcano", "howls", "liar", "egotistic", "befit", "rounded", "older", "bluntly", "imbalance", "pivot", "exotic", "tuxedo", "amaze", "mostly", "lukewarm", "macro", "vocal", "hounded", "biplane", "rounded"].iter().map(|s| s.to_string()).collect();
/// let hex_seed: String = derive_hex_seed(mnemonic);
/// assert_eq!(hex_seed, "f7b3beabc9bd6ced864096c0891a8fdf94dc714178a09828775dba01b4df9ab8".to_string());
/// ```
pub fn derive_hex_seed(mut mnemonic_seed: Vec<String>) -> String {
    // Find the wordset for the given seed
    let mut the_wordset = &WordsetOriginal {
        name: "x",
        prefix_len: 0,
        words: [""; 1626],
    };
    for wordset in WORDSETSORIGINAL.iter() {
        if mnemonic_seed
            .iter()
            .all(|elem| wordset.words.contains(&elem.as_str()))
        {
            the_wordset = wordset;
            break;
        }
    }
    if the_wordset.name == "x" {
        panic!("Wordset could not be found for given seed, please check your seed");
    }

    // Remove checksum word
    if the_wordset.prefix_len > 0 {
        mnemonic_seed.pop();
    }

    // Get a vector of truncated words
    let mut trunc_words: Vec<&str> = Vec::new();
    for word in the_wordset.words.iter() {
        trunc_words.push(&word[..the_wordset.prefix_len]);
    }
    if trunc_words.is_empty() {
        panic!("Something went wrong when decoding your private key, please try again");
    }

    // Derive hex seed
    let mut hex_seed = String::new();
    let wordset_len: usize = the_wordset.words.len();
    for i in (0..mnemonic_seed.len()).step_by(3) {
        let (w1, w2, w3): (usize, usize, usize);
        if the_wordset.prefix_len == 0 {
            w1 = the_wordset
                .words
                .iter()
                .position(|&x| x == mnemonic_seed[i])
                .unwrap_or_else(|| panic!("Invalid word in seed, please check your seed"));
            w2 = the_wordset
                .words
                .iter()
                .position(|&x| x == mnemonic_seed[i + 1])
                .unwrap_or_else(|| panic!("Invalid word in seed, please check your seed"));
            w3 = the_wordset
                .words
                .iter()
                .position(|&x| x == mnemonic_seed[i + 2])
                .unwrap_or_else(|| panic!("Invalid word in seed, please check your seed"));
        } else {
            w1 = trunc_words
                .iter()
                .position(|&x| x.starts_with(&mnemonic_seed[i][..the_wordset.prefix_len]))
                .unwrap_or_else(|| panic!("Invalid word in seed, please check your seed"));
            w2 = trunc_words
                .iter()
                .position(|&x| x.starts_with(&mnemonic_seed[i + 1][..the_wordset.prefix_len]))
                .unwrap_or_else(|| panic!("Invalid word in seed, please check your seed"));
            w3 = trunc_words
                .iter()
                .position(|&x| x.starts_with(&mnemonic_seed[i + 2][..the_wordset.prefix_len]))
                .unwrap_or_else(|| panic!("Invalid word in seed, please check your seed"));
        }

        let x = w1
            + wordset_len * (((wordset_len - w1) + w2) % wordset_len)
            + wordset_len * wordset_len * (((wordset_len - w2) + w3) % wordset_len);
        if x % wordset_len != w1 {
            panic!("Something went wrong when decoding your private key, please try again");
        }

        hex_seed += &swap_endian_4_byte(&format!("{:08x}", x));
    }

    hex_seed
}

/// Derives private keys for original (25-word) (64-byte hex) type seeds
fn derive_original_priv_keys(hex_seed: String) -> Vec<String> {
    // Turn hex seed into bytes
    let hex_bytes = hex::decode(hex_seed).unwrap();
    let mut hex_bytes_array = [0u8; 32];
    hex_bytes_array.copy_from_slice(&hex_bytes);
    // Pass bytes through sc_reduce32 function to get private spend key
    sc_reduce32(&mut hex_bytes_array);
    let mut priv_spend_key = String::new();
    for i in (0..hex_bytes_array.len()).step_by(32) {
        let mut priv_key = String::new();
        for byte in hex_bytes_array.iter().skip(i).take(32) {
            priv_key.push_str(&format!("{:02x}", byte));
        }
        priv_spend_key.push_str(&priv_key);
    }
    // Turn private spend key into bytes and pass through Keccak256 function
    let priv_spend_key_bytes = hex::decode(priv_spend_key.clone()).unwrap();
    let priv_view_key_bytes = Keccak256::digest(priv_spend_key_bytes);
    let mut priv_view_key_array = [0u8; 32];
    priv_view_key_array.copy_from_slice(&priv_view_key_bytes);
    // Pass bytes through sc_reduce32 function to get private view key
    sc_reduce32(&mut priv_view_key_array as &mut [u8; 32]);
    let mut priv_view_key = String::new();
    for i in (0..priv_view_key_array.len()).step_by(32) {
        let mut priv_key = String::new();
        for byte in priv_view_key_array.iter().skip(i).take(32) {
            priv_key.push_str(&format!("{:02x}", byte));
        }
        priv_view_key.push_str(&priv_key);
    }
    // Finally, return the keys
    vec![priv_spend_key, priv_view_key]
}

/// Derives private keys for MyMonero (13-word) (32-byte hex) type seeds
fn derive_mymonero_priv_keys(hex_seed: String) -> Vec<String> {
    // Keccak and sc_reduce32 to get private spend key
    let hex_bytes = hex::decode(hex_seed).unwrap();
    let priv_spend_key_bytes = Keccak256::digest(&hex_bytes);
    let mut priv_spend_key_array = [0u8; 32];
    priv_spend_key_array.copy_from_slice(&priv_spend_key_bytes);
    sc_reduce32(&mut priv_spend_key_array as &mut [u8; 32]);
    let mut priv_spend_key = String::new();
    for i in (0..priv_spend_key_array.len()).step_by(32) {
        let mut priv_key = String::new();
        for item in priv_spend_key_array.iter().skip(i).take(32) {
            priv_key.push_str(&format!("{:02x}", item));
        }
        priv_spend_key.push_str(&priv_key);
    }
    // Double Keccak and sc_reduce32 of hex_seed to get private view key
    let priv_view_key_bytes = Keccak256::digest(&hex_bytes);
    let mut priv_view_key_array = [0u8; 32];
    priv_view_key_array.copy_from_slice(&priv_view_key_bytes);
    // Keccak again
    let priv_view_key_bytes = Keccak256::digest(priv_view_key_array);
    priv_view_key_array.copy_from_slice(&priv_view_key_bytes);
    // sc_reduce32
    sc_reduce32(&mut priv_view_key_array as &mut [u8; 32]);
    let mut priv_view_key = String::new();
    for i in (0..priv_view_key_array.len()).step_by(32) {
        let mut priv_key = String::new();
        for item in priv_view_key_array.iter().skip(i).take(32) {
            priv_key.push_str(&format!("{:02x}", item));
        }
        priv_view_key.push_str(&priv_key);
    }
    // Finally, return the keys
    vec![priv_spend_key, priv_view_key]
}

/// Derives private keys from given hex seed
///
/// Vector's first element is private spend key, second element is private view key
///
/// Example:
/// ```
/// use libmonero::keys::derive_priv_keys;
///
/// let hex_seed: String = "f7b3beabc9bd6ced864096c0891a8fdf94dc714178a09828775dba01b4df9ab8".to_string();
/// let priv_keys: Vec<String> = derive_priv_keys(hex_seed);
/// assert_eq!(priv_keys, vec!["c8982eada77ba2245183f2bff85dfaf993dc714178a09828775dba01b4df9a08", "0d13a94c82d7a60abb54d2217d38935c3f715295e30378f8848a1ca1abc8d908"].iter().map(|&s| s.to_string()).collect::<Vec<String>>());
/// ```
pub fn derive_priv_keys(hex_seed: String) -> Vec<String> {
    match hex_seed.len() {
        32 => derive_mymonero_priv_keys(hex_seed),
        64 => derive_original_priv_keys(hex_seed),
        _ => panic!("Invalid hex seed"),
    }
}

/// Derives private view key from given private spend key
///
/// Example:
/// ```
/// use libmonero::keys::derive_priv_vk_from_priv_sk;
///
/// let private_spend_key: String = "c8982eada77ba2245183f2bff85dfaf993dc714178a09828775dba01b4df9a08".to_string();
/// let private_view_key: String = derive_priv_vk_from_priv_sk(private_spend_key);
/// assert_eq!(private_view_key, "0d13a94c82d7a60abb54d2217d38935c3f715295e30378f8848a1ca1abc8d908".to_string());
/// ```
pub fn derive_priv_vk_from_priv_sk(private_spend_key: String) -> String {
    // Turn private spend key into bytes and pass through Keccak256 function
    let priv_spend_key_bytes = hex::decode(private_spend_key.clone()).unwrap();
    let priv_view_key_bytes = Keccak256::digest(priv_spend_key_bytes);
    let mut priv_view_key_array = [0u8; 32];
    priv_view_key_array.copy_from_slice(&priv_view_key_bytes);
    // Pass bytes through sc_reduce32 function to get private view key
    sc_reduce32(&mut priv_view_key_array as &mut [u8; 32]);
    let mut priv_view_key = String::new();
    for i in (0..priv_view_key_array.len()).step_by(32) {
        let mut priv_key = String::new();
        for item in priv_view_key_array.iter().skip(i).take(32) {
            priv_key.push_str(&format!("{:02x}", item));
        }
        priv_view_key.push_str(&priv_key);
    }
    // Finally, return the private view key
    priv_view_key
}

/// Performs scalar multiplication of the Ed25519 base point by a given scalar, yielding a corresponding point on the elliptic curve
fn ge_scalar_mult_base(scalar: &Scalar) -> EdwardsPoint {
    ED25519_BASEPOINT_TABLE.mul(scalar as &Scalar)
}

/// Derives public key from given private key (spend or view)
///
/// Example:
/// ```
/// use libmonero::keys::derive_pub_key;
///
/// let private_spend_key: String = "c8982eada77ba2245183f2bff85dfaf993dc714178a09828775dba01b4df9a08".to_string();
/// let public_spend_key: String = derive_pub_key(private_spend_key);
/// assert_eq!(public_spend_key, "e78d891dd2be407f24e6470caad956e1b746ae0b41cd8252f96684090bc05d95".to_string());
/// ```
pub fn derive_pub_key(private_key: String) -> String {
    // Turn private key into bytes
    let private_key_bytes = hex::decode(private_key.clone()).unwrap();
    let mut private_key_array = [0u8; 32];
    private_key_array.copy_from_slice(&private_key_bytes);
    let key_scalar = Scalar::from_bytes_mod_order(private_key_array);
    // Scalar multiplication with the base point
    let result_point = ge_scalar_mult_base(&key_scalar);
    // The result_point now contains the public key
    let public_key_bytes = result_point.compress().to_bytes();
    let mut public_key = String::new();
    for i in (0..public_key_bytes.len()).step_by(32) {
        let mut pub_key = String::new();
        for item in public_key_bytes.iter().skip(i).take(32) {
            pub_key.push_str(&format!("{:02x}", item));
        }
        public_key.push_str(&pub_key);
    }
    // Finally, return the public key
    public_key
}

/// Derives main public address from given public spend key, public view key and network
///
/// Networks:
/// - `0` : Monero Mainnet
/// - `1` : Monero Testnet
///
/// Example:
/// ```
/// use libmonero::keys::derive_address;
///
/// let public_spend_key: String = "e78d891dd2be407f24e6470caad956e1b746ae0b41cd8252f96684090bc05d95".to_string();
/// let public_view_key: String = "157d278aa3aee4e11c5a8243a43a78527a2691009562b8c18654975f1347cb47".to_string();
/// let public_address: String = derive_address(public_spend_key, public_view_key, 0);
/// assert_eq!(public_address, "4AQ3jTJg91yNGTXjo9iWr1ekjBGJ5mM6HEsxKqoKddHnRwJTVJYnyLXeerff6iTys5Eo8dyG87tfqZNS5CcSd7U694YiR8J".to_string());
/// ```
pub fn derive_address(public_spend_key: String, public_view_key: String, network: u8) -> String {
    let network_byte = match network {
        0 => vec![0x12], // Monero mainnet
        1 => vec![0x35], // Monero testnet
        _ => panic!("Invalid network"),
    };
    let pub_sk_bytes = hex::decode(public_spend_key.clone()).unwrap();
    let pub_vk_bytes = hex::decode(public_view_key.clone()).unwrap();
    let mut data = [&network_byte[..], &pub_sk_bytes[..], &pub_vk_bytes[..]].concat();
    let hash = Keccak256::digest(&data);
    data.append(&mut hash[..4].to_vec());

    base58_monero::encode(&data).unwrap()
}
