use std::str::FromStr;

use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G2Affine};
use ark_groth16::Proof;
use shielded_pool::verifier::verify;

fn test_proof() -> Proof<Bn254> {
    let fq = |s: &str| Fq::from_str(s).unwrap();
    Proof {
        a: G1Affine::new(
            fq("15888145343833448239953459943024321853771690511882159769893749765564864396445"),
            fq("8589097242134644594835363874760996910162850348919755575117850180138769432658"),
        ),
        b: G2Affine::new(
            Fq2::new(
                fq("5326089907765973646048352806539136199074074135130682865437290567984032177026"),
                fq("6776452530066191603983218177527908574474543437908482670615691992019955889970"),
            ),
            Fq2::new(
                fq("5583632982302398612395106981011682726540162344817368627046607371891584624356"),
                fq("10457261946597323870322016386055696111107784437919413747023996451058687091362"),
            ),
        ),
        c: G1Affine::new(
            fq("6903754294212080435567652967895232878661520880546242918013948578106771784061"),
            fq("21651657613394627992711352621973794478676691445279362131448711971388155711737"),
        ),
    }
}

fn test_inputs() -> Vec<Fr> {
    [
        "3045043903935407917568106036056436390653602748722528304335468615555843441317",
        "7461709316779476527177526933769744953370949361207311192114610578379086861367",
        "5489321352117474093008004637687255696673429846584932393966876642221060920346",
        "0",
        "0",
        "0",
    ]
    .iter()
    .map(|s| Fr::from_str(s).unwrap())
    .collect()
}

#[test]
fn valid_proof_verifies() {
    assert_eq!(verify(&test_proof(), &test_inputs()).unwrap(), true);
}

#[test]
fn wrong_inputs_rejected() {
    let mut inputs = test_inputs();
    inputs[0] = Fr::from(42u64);
    assert_eq!(verify(&test_proof(), &inputs).unwrap(), false);
}

#[test]
fn mutated_proof_rejected() {
    let mut proof = test_proof();
    std::mem::swap(&mut proof.a, &mut proof.c);
    assert_eq!(verify(&proof, &test_inputs()).unwrap(), false);
}
