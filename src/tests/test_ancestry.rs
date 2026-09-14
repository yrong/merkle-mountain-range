use super::{MergeNumberHash, NumberHash};
use crate::ancestry_proof::{ancestry_proof_positions, expected_ancestry_proof_size};
use crate::leaf_index_to_mmr_size;
use crate::util::{MemMMR, MemStore};
use crate::Error;

#[test]
fn test_ancestry() {
    let store = MemStore::default();
    let mut mmr = MemMMR::<_, MergeNumberHash>::new(0, &store);

    let mmr_size = 5000;
    let mut prev_roots = Vec::new();
    for i in 0..mmr_size {
        mmr.push(NumberHash::from(i)).unwrap();
        prev_roots.push(mmr.get_root().expect("get root"));
    }

    let root = mmr.get_root().expect("get root");
    for i in 0..mmr_size {
        let prev_size = leaf_index_to_mmr_size(i.into());
        let ancestry_proof = mmr.gen_ancestry_proof(prev_size).expect("gen proof");
        assert!(ancestry_proof
            .verify_ancestor(root.clone(), prev_roots[i as usize].clone())
            .unwrap());
        assert_eq!(
            expected_ancestry_proof_size(ancestry_proof.prev_mmr_size, mmr.mmr_size()),
            ancestry_proof.prev_peaks_proof.proof_items().len()
        );
    }
}

/// `ancestry_proof_positions` must name exactly the positions `gen_ancestry_proof` emits, in the
/// same order, for every `(prev, current)` pair — including the item that stands for a bagged
/// run of right-hand peaks. Exhaustive up to 256 leaves, which covers the all-ones sizes 127 and
/// 255 (seven and eight peaks, the most bagging the range allows).
#[test]
fn test_ancestry_proof_positions_match_generated_proof() {
    for n in 2..=256u64 {
        let store = MemStore::default();
        let mut mmr = MemMMR::<_, MergeNumberHash>::new(0, &store);
        for i in 0..n {
            mmr.push(NumberHash::from(i as u32)).unwrap();
        }
        let mmr_size = mmr.mmr_size();
        for k in 1..n {
            let prev_size = leaf_index_to_mmr_size(k - 1);
            let proof = mmr.gen_ancestry_proof(prev_size).expect("gen proof");
            let generated: Vec<u64> = proof
                .prev_peaks_proof
                .proof_items()
                .iter()
                .map(|(pos, _)| *pos)
                .collect();
            let derived = ancestry_proof_positions(prev_size, mmr_size).expect("positions");
            assert_eq!(derived, generated, "k={k} n={n}");
            assert_eq!(
                derived.len(),
                expected_ancestry_proof_size(prev_size, mmr_size)
            );
        }
    }
}

#[test]
fn test_ancestry_proof_positions_rejects_what_gen_ancestry_proof_rejects() {
    // An empty previous MMR has no peaks to prove.
    assert!(ancestry_proof_positions(0, 7).is_err());
    // A previous MMR larger than the current one has peaks outside it.
    assert!(ancestry_proof_positions(7, 4).is_err());
    // The single-leaf identity case: nothing to prove.
    assert_eq!(ancestry_proof_positions(1, 1).unwrap(), Vec::<u64>::new());
    // Sizes that are not MMR sizes are rejected the way the verifiers reject them.
    assert!(matches!(
        ancestry_proof_positions(2, 7),
        Err(Error::CorruptedProof)
    ));
    assert!(matches!(
        ancestry_proof_positions(3, 5),
        Err(Error::CorruptedProof)
    ));
}
