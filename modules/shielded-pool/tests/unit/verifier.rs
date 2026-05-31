use std::str::FromStr;

use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G2Affine};
use ark_groth16::Proof;
use shielded_pool::verifier::verify;

fn test_proof() -> Proof<Bn254> {
    let fq = |s: &str| Fq::from_str(s).unwrap();
    Proof {
        a: G1Affine::new(
            fq("5962834336838907434183654712061717918330074867986010160572231458372658040209"),
            fq("16102888246426368429584038978640737772248977239956282462595542580231678386433"),
        ),
        b: G2Affine::new(
            Fq2::new(
                fq("17465664610970982255772415852413466604714912645632154897354761400370125304135"),
                fq("18948308574138177316176743899259456587619588033995855301276009378982058999177"),
            ),
            Fq2::new(
                fq("574379083950143117150059523551397838191082795935161913793534882121273098764"),
                fq("7940314523098736593680336485643124487508803635622763663736115560822094495804"),
            ),
        ),
        c: G1Affine::new(
            fq("12300672075698487053624550960151118747751056785430719002017329433595921793311"),
            fq("9532349455619607556679827659506439964107712832207700528970710858410377840677"),
        ),
    }
}

fn test_inputs() -> Vec<Fr> {
    [
        "5710971321842944456102627447350829862940489574472045465791465949445609370008",
        "8471445357757317028257194288389064310025407834984015450349261049729116171716",
        "4950032633621571813633625100838819026132126256338564784592571389228323516374",
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
