//! Binary to generate hardcoded parameters for EvaluationDomain supported in `PolynomialEval.sol`

use ark_bn254::Fr;
use ark_ff::{Field, PrimeField};
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};

fn main() {
    let domain = Radix2EvaluationDomain::<Fr>::new(2u32.pow(20) as usize).unwrap();

    let size_inv = <<Fr as Field>::BasePrimeField as PrimeField>::into_bigint(domain.size_inv);
    let size_inv_str = format!("0x{size_inv:X}").to_lowercase();
    println!(
        "domain logSize: {}, size: {}, sizeInv: {}",
        domain.log_size_of_group, domain.size, size_inv_str
    );

    let group_gen = <<Fr as Field>::BasePrimeField as PrimeField>::into_bigint(domain.group_gen);
    let group_gen_inv =
        <<Fr as Field>::BasePrimeField as PrimeField>::into_bigint(domain.group_gen_inv);
    let group_gen_str = format!("0x{group_gen:X}").to_lowercase();
    let group_gen_inv_str = format!("0x{group_gen_inv:X}").to_lowercase();
    println!(
        "groupGen: {group_gen_str}, groupGenInv: {group_gen_inv_str}",
    );

    let mut domain_elements_str = "".to_owned();

    // Generates the domain elements: 1, g, g^2,...,g^10
    for i in 0..=10 {
        let mut element_fr = domain.group_gen;
        element_fr = element_fr.pow([i]);

        let element_bigint = element_fr.into_bigint();
        let new_element_str = format!("0x{:X} ", element_bigint.clone()).to_lowercase();
        domain_elements_str = domain_elements_str.to_owned() + &new_element_str;
    }
    println!("domain elements: {}", &domain_elements_str);
}
