/*
 * This file is part of Monume's library libmonero
 *
 * Copyright (c) 2023 Monume
 * All Rights Reserved
 * The code is distributed under MIT license, see LICENSE file for details.
 * Generated by Monume
 *
 */

use crate::ed25519::sc_reduce32;
use crate::mnemonics::{Wordset1626, WORDSETS1626};
use crc32fast::Hasher;
use curve25519_dalek::constants::ED25519_BASEPOINT_TABLE;
use curve25519_dalek::{EdwardsPoint, Scalar};
use rand::{rngs::StdRng, Rng, SeedableRng};
use sha3::{Digest, Keccak256};
use std::convert::TryFrom;
use std::ops::Mul;

// Returns cryptographically secure random element of the given array
fn secure_random_element<'x>(array: &'x [&'x str]) -> &'x str {
    let seed: [u8; 32] = rand::thread_rng().gen();
    let mut rng = StdRng::from_seed(seed);
    let index = rng.gen_range(0..array.len());
    array[index]
}

// Calculates CRC32 checksum index for given array (probably the seed)
fn get_checksum_index(array: &[&str], prefix_length: usize) -> usize {
    let mut trimmed_words: String = String::new();
    for word in array {
        trimmed_words.push_str(&word[0..prefix_length]);
    }
    let mut hasher = Hasher::new();
    hasher.update(trimmed_words.as_bytes());
    usize::try_from(hasher.finalize()).unwrap() % array.len()
}

// Generates a cryptographically secure 1626-word type seed for given language
fn generate1626seed(language: &str) -> Vec<&str> {
    let mut seed: Vec<&str> = Vec::new();
    let mut prefix_len: usize = 3;
    for wordset in WORDSETS1626.iter() {
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
    if seed.is_empty() {
        panic!("Language not found");
    }
    // Add checksum word
    let checksum_index = get_checksum_index(&seed, prefix_len);
    seed.push(seed[checksum_index]);
    // Finally, return the seed
    seed
}

// Creates a cryptographically secure seed of given type and language
pub fn generate_seed<'a>(language: &'a str, seed_type: &'a str) -> Vec<&'a str> {
    match seed_type {
        "1626" => generate1626seed(language),
        "polyseed" => panic!("Polyseed not implemented yet"),
        _ => panic!("Invalid seed type"),
    }
}

// Swaps endianness of a 4-byte string
fn swap_endian_4_byte(s: &str) -> String {
    if s.len() != 8 {
        panic!("Invalid length of string");
    }
    format!("{}{}{}{}", &s[6..8], &s[4..6], &s[2..4], &s[0..2])
}

// Finds index of a given word in a given array
fn find_index(array: &[&str], word: &str) -> isize {
    array
        .iter()
        .position(|&x| x == word)
        .map(|i| i as isize)
        .unwrap_or(-1)
}

// Derives hex seed from given mnemonic seed
pub fn derive_hex_seed(mut mnemonic_seed: Vec<&str>) -> String {
    // Find the wordset for the given seed
    let mut the_wordset = &Wordset1626 {
        name: "invalid",
        prefix_len: 0,
        words: [""; 1626],
    }; // This is given for checking in future if the wordset was found
    for wordset in WORDSETS1626.iter() {
        for word in wordset.words.iter() {
            if mnemonic_seed.contains(word) {
                the_wordset = wordset;
                break;
            }
        }
    }
    if the_wordset.name == "invalid" {
        panic!("The wordset could not be found for given seed, please check your seed")
    }

    // Declare variables for later use
    let mut hex_seed = String::new();
    let ws_word_len = the_wordset.words.len();
    let mut checksum_word = String::new();

    // Check if seed is valid
    if (the_wordset.prefix_len == 0 && mnemonic_seed.len() % 3 != 0)
        || (the_wordset.prefix_len > 0 && mnemonic_seed.len() % 3 == 2)
    {
        panic!("You have entered too few words, please check your seed")
    } else if the_wordset.prefix_len > 0 && mnemonic_seed.len() % 3 == 0 {
        panic!("You seem to be missing the last word of your seed, please check your seed")
    } else if the_wordset.prefix_len > 0 {
        checksum_word = mnemonic_seed.pop().unwrap().to_string();
    }

    // Get list of truncated words
    let mut trunc_words: Vec<&str> = Vec::new();
    if the_wordset.prefix_len > 0 {
        for word in the_wordset.words.iter() {
            trunc_words.push(&word[..the_wordset.prefix_len]);
        }
    }

    // Derive hex seed
    for i in (0..mnemonic_seed.len()).step_by(3) {
        let w1;
        let w2;
        let w3;
        if the_wordset.prefix_len == 0 {
            w1 = find_index(&the_wordset.words, mnemonic_seed[i]);
            w2 = find_index(&the_wordset.words, mnemonic_seed[i + 1]);
            w3 = find_index(&the_wordset.words, mnemonic_seed[i + 2]);
        } else {
            w1 = find_index(&trunc_words, &mnemonic_seed[i][..the_wordset.prefix_len]);
            w2 = find_index(
                &trunc_words,
                &mnemonic_seed[i + 1][..the_wordset.prefix_len],
            );
            w3 = find_index(
                &trunc_words,
                &mnemonic_seed[i + 2][..the_wordset.prefix_len],
            );
        }

        if w1 == -1 || w2 == -1 || w3 == -1 {
            panic!("Invalid word in seed, please check your seed")
        }

        let x: usize = (w1
            + ws_word_len as isize * ((ws_word_len as isize - w1 + w2) % ws_word_len as isize)
            + ws_word_len as isize
                * ws_word_len as isize
                * ((ws_word_len as isize - w2 + w3) % ws_word_len as isize))
            .try_into()
            .unwrap();
        if x % ws_word_len != w1 as usize {
            panic!("An error occured while deriving hex seed, please try again later");
        }
        let swapped = swap_endian_4_byte(&format!("{:08x}", x));
        hex_seed += &swapped;
    }

    // Verify checksum
    if the_wordset.prefix_len > 0 {
        let index = get_checksum_index(&mnemonic_seed, the_wordset.prefix_len);
        let expected_checksum_word = &mnemonic_seed[index];
        if expected_checksum_word[..the_wordset.prefix_len]
            != checksum_word[..the_wordset.prefix_len]
        {
            panic!("Your seed could not be verified via the last word checksum, please check your seed")
        }
    }
    // Finally, return the hex seed
    hex_seed
}

// Derives private spend and view keys from given hex seed
pub fn derive_priv_keys(hex_seed: String) -> Vec<String> {
    // Turn hex seed into bytes
    let hex_bytes = hex::decode(hex_seed).unwrap();
    let mut hex_bytes_array = [0u8; 32];
    hex_bytes_array.copy_from_slice(&hex_bytes);
    // Pass bytes through sc_reduce32 function to get private spend key
    sc_reduce32(&mut hex_bytes_array);
    let mut priv_spend_key = String::new();
    for i in (0..hex_bytes_array.len()).step_by(32) {
        let mut priv_key = String::new();
        for j in i..i + 32 {
            priv_key.push_str(&format!("{:02x}", hex_bytes_array[j]));
        }
        priv_spend_key.push_str(&priv_key);
    }
    // Turn private spend key into bytes and pass through Keccak256 function
    let priv_spend_key_bytes = hex::decode(priv_spend_key.clone()).unwrap();
    let priv_view_key_bytes = Keccak256::digest(&priv_spend_key_bytes);
    let mut priv_view_key_array = [0u8; 32];
    priv_view_key_array.copy_from_slice(&priv_view_key_bytes);
    // Pass bytes through sc_reduce32 function to get private view key
    sc_reduce32(&mut priv_view_key_array as &mut [u8; 32]);
    let mut priv_view_key = String::new();
    for i in (0..priv_view_key_array.len()).step_by(32) {
        let mut priv_key = String::new();
        for j in i..i + 32 {
            priv_key.push_str(&format!("{:02x}", priv_view_key_array[j]));
        }
        priv_view_key.push_str(&priv_key);
    }
    // Finally, return the keys
    vec![priv_spend_key, priv_view_key]
}

// Derives private view key from private spend key
pub fn derive_priv_vk_from_priv_sk(private_spend_key: String) -> String {
    // Turn private spend key into bytes and pass through Keccak256 function
    let priv_spend_key_bytes = hex::decode(private_spend_key.clone()).unwrap();
    let priv_view_key_bytes = Keccak256::digest(&priv_spend_key_bytes);
    let mut priv_view_key_array = [0u8; 32];
    priv_view_key_array.copy_from_slice(&priv_view_key_bytes);
    // Pass bytes through sc_reduce32 function to get private view key
    sc_reduce32(&mut priv_view_key_array as &mut [u8; 32]);
    let mut priv_view_key = String::new();
    for i in (0..priv_view_key_array.len()).step_by(32) {
        let mut priv_key = String::new();
        for j in i..i + 32 {
            priv_key.push_str(&format!("{:02x}", priv_view_key_array[j]));
        }
        priv_view_key.push_str(&priv_key);
    }
    // Finally, return the private view key
    priv_view_key
}

// Performs scalar multiplication of the Ed25519 base point by a given scalar, yielding a corresponding point on the elliptic curve
fn ge_scalar_mult_base(scalar: &Scalar) -> EdwardsPoint {
    // Scalar multiplication with the base point
    let result_point = ED25519_BASEPOINT_TABLE.mul(scalar as &Scalar);
    // The result_point now contains the public key
    result_point
}

// Derives public key from given private key, can be either spend or view key
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
        for j in i..i + 32 {
            pub_key.push_str(&format!("{:02x}", public_key_bytes[j]));
        }
        public_key.push_str(&pub_key);
    }
    // Finally, return the public key
    public_key
}

// Derives public address from given public spend and view keys
pub fn derive_address(public_spend_key: String, public_view_key: String, network: i8) -> String {
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
    let address = base58_monero::encode(&data).unwrap();
    address
}