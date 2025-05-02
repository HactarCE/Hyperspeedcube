import "utils" as utils;

const REALISITIC_PROPORTIONS = true;
const CORNER_STALK_SIZE = 0.1;

fn ft_cube_cut_depths(ndim, layers) {
    if layers < 2 {
        throw `facet-turning cube cannot have ${layers} layers`;
    }

    let aesthetic_limit = 1 - 2/layers;
    let mechanical_limit = 0;
    if global::REALISITIC_PROPORTIONS {
        mechanical_limit = 1 / sqrt(ndim-1);
    }
    let outermost_cut = min(aesthetic_limit, mechanical_limit - global::CORNER_STALK_SIZE);
    utils::layers::inclusive_inf(outermost_cut, -outermost_cut, layers)
}
