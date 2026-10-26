//! Differential test: `rheaps` 0.16.1's pairing heap must pop equal keys in the same order as
//! our JHeaps-faithful `JavaPairingHeap` (verified against the real JHeaps 0.15 jar by replaying a
//! 2000-step insert/pop/decrease-key script). Plain insert/pop agree; decrease-key and delete do not.

use jgrapht::util::heap::JavaPairingHeap;
use rand::{Rng, SeedableRng, rngs::StdRng};
use rheaps::tree::PairingHeap;
use rheaps::{AddressableHeap, DecreaseKeyHeap};
fn run(allow_dec: bool, allow_del: bool) {
    for seed in 0..300u64 {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut a = PairingHeap::<i32, u32>::new();
        let mut b = JavaPairingHeap::<i32, u32>::new();
        let (mut ha, mut hb) = (Vec::new(), Vec::new());
        let mut next = 0u32;
        for _ in 0..400 {
            match rng.random_range(0..10) {
                0..=4 => {
                    let k = rng.random_range(0..6); // few distinct keys -> lots of ties
                    ha.push(Some(a.insert(k, next)));
                    hb.push(Some(b.insert(k, next)));
                    next += 1;
                }
                5..=6 => {
                    let (x, y) = (a.pop(), b.pop());
                    assert_eq!(x, y, "seed {seed}");
                    if let Some((_, v)) = x {
                        ha[v as usize] = None;
                        hb[v as usize] = None;
                    }
                }
                7..=8 if allow_dec => {
                    let live: Vec<usize> = (0..ha.len()).filter(|&i| ha[i].is_some()).collect();
                    if live.is_empty() {
                        continue;
                    }
                    let i = live[rng.random_range(0..live.len())];
                    let cur = *a.key(ha[i].unwrap()).unwrap();
                    let nk = cur - rng.random_range(0..3);
                    assert_eq!(a.decrease_key(ha[i].unwrap(), nk).is_ok(), b.decrease_key(hb[i].unwrap(), nk).is_ok());
                }
                9 if allow_del => {
                    let live: Vec<usize> = (0..ha.len()).filter(|&i| ha[i].is_some()).collect();
                    if live.is_empty() {
                        continue;
                    }
                    let i = live[rng.random_range(0..live.len())];
                    assert_eq!(a.delete(ha[i].unwrap()).unwrap(), b.delete(hb[i].unwrap()).unwrap(), "seed {seed}");
                    ha[i] = None;
                    hb[i] = None;
                }
                _ => {}
            }
        }
        loop {
            let (x, y) = (a.pop(), b.pop());
            assert_eq!(x, y, "seed {seed}");
            if x.is_none() {
                break;
            }
        }
    }
}

#[test]
fn insert_pop_only() {
    run(false, false);
}
#[test]
#[ignore = "rheaps 0.16.1 cut_from_parent uses swap_remove, which reorders siblings; JHeaps keeps their order"]
fn with_decrease_key() {
    run(true, false);
}
#[test]
#[ignore = "same cause as with_decrease_key"]
fn with_delete() {
    run(false, true);
}
