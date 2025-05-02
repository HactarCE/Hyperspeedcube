use hypermath::IndexNewtype;
use hypermath::collections::GenericVec;
use hyperpuzzle_core::{NameSpec, NameSpecBiMapBuilder};
use hypershape::GenSeq;
use itertools::Itertools;
use rhai::Array;

use super::*;

/// Constructs an assignment of names based on a map for a particular
/// symmetry group.
pub fn names_from_map<I: IndexNewtype>(ctx: &Ctx<'_>, map: Map) -> Result<Vec<(GenSeq, NameSpec)>> {
    let mut names = NameSpecBiMapBuilder::<I>::new();
    let mut gen_seqs = GenericVec::new();
    for (k, v) in map {
        let mut seq: Array = from_rhai(ctx, v)?;

        let init_name: Option<String> = from_rhai_opt(ctx, seq.pop_if(|v| v.is_string()))?;
        let gen_seq: GenSeq = seq.into_iter().map(|v| from_rhai(ctx, v)).try_collect()?;
        let id = gen_seqs
            .push((gen_seq, init_name))
            .map_err(|e| e.to_string())?;

        let name_spec: String = k.into();
        names.set(id, Some(name_spec)).map_err(|e| e.to_string())?;
    }

    // Resolve string names to IDs.
    let key_value_dependencies: GenericVec<I, (GenSeq, Option<I>)> =
        gen_seqs.try_map(|_id, (gen_seq, end)| -> Result<_> {
            let new_end = match end {
                Some(ending_name) => {
                    let opt_id = names.id_from_string(&ending_name);
                    Some(opt_id.ok_or_else(|| format!("no name matches {ending_name:?}"))?)
                }
                None => None,
            };
            Ok((gen_seq, new_end))
        })?;

    // Resolve lazy evaluation.
    Ok(hyperpuzzle_core::util::lazy_resolve(
        key_value_dependencies,
        |mut seq1, seq2| {
            // TODO: O(n^2)
            seq1.0.extend_from_slice(&seq2.0);
            seq1
        },
        warnf(ctx),
    )
    .into_iter()
    .filter_map(|(k, v)| Some((v, names.get(k)?.clone())))
    .collect())
}
