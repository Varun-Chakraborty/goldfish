use std::cmp::max;

static PASSED_PAWN_SCORE: &[i32; 6] = &[0, 10, 20, 35, 60, 100];
const ISOLATED_PAWN_PENALTY: i32 = 15;

pub struct PawnStructure {
    pub passed: i32,
    pub isolated: i32,
    pub doubled: i32,
}

pub fn pawn_structure(wp: &[u8; 8], bp: &[u8; 8]) -> PawnStructure {
    let mut isolated = 0;
    let mut doubled = 0;
    let mut passed = 0;

    for (i, file) in wp.iter().enumerate() {
        doubled -= max(0, file.count_ones() as i32 - 1);
        if *file > 0
            && i.checked_sub(1)
                .is_none_or(|i| wp.get(i).is_none_or(|file| *file == 0))
            && wp.get(i + 1).is_none_or(|file| *file == 0)
        {
            isolated -= ISOLATED_PAWN_PENALTY * file.count_ones() as i32;
        }

        let mut is_passed = false;
        for rank in 1..7 {
            let p = file >> rank & 1;
            if p == 0 {
                continue;
            }

            if is_passed {
                passed += PASSED_PAWN_SCORE[rank - 1];
            } else if i
                .checked_sub(1)
                .is_none_or(|i| bp.get(i).is_none_or(|file| file >> rank == 0))
                && bp.get(i).is_none_or(|file| file >> rank == 0)
                && bp.get(i + 1).is_none_or(|file| file >> rank == 0)
            {
                is_passed = true;
                passed += PASSED_PAWN_SCORE[rank - 1];
            }
        }
    }

    for (i, file) in bp.iter().enumerate() {
        doubled += max(0, file.count_ones() as i32 - 1);
        if *file > 0
            && i.checked_sub(1)
                .is_none_or(|i| bp.get(i).is_none_or(|file| *file == 0))
            && bp.get(i + 1).is_none_or(|file| *file == 0)
        {
            isolated += ISOLATED_PAWN_PENALTY * file.count_ones() as i32;
        }

        let mut is_passed = false;
        for rank in 1..7 {
            let p = file >> (7 - rank) & 1;
            if p == 0 {
                continue;
            }
            if is_passed {
                passed -= PASSED_PAWN_SCORE[6 - rank];
            } else if i
                .checked_sub(1)
                .is_none_or(|i| wp.get(i).is_none_or(|file| file << rank == 0))
                && wp.get(i).is_none_or(|file| file << rank == 0)
                && wp.get(i + 1).is_none_or(|file| file << rank == 0)
            {
                is_passed = true;
                passed -= PASSED_PAWN_SCORE[6 - rank];
            }
        }
    }

    PawnStructure {
        passed,
        isolated,
        doubled,
    }
}
