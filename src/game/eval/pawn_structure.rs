use std::cmp::max;

static PASSED_PAWN_SCORE: &[i32; 6] = &[0, 10, 20, 35, 60, 100];
const ISOLATED_PAWN_PENALTY: i32 = 15;

pub struct PawnStructure {
    pub passed: i32,
    pub isolated: i32,
    pub doubled: i32,
}

pub fn pawn_structure(wp: u64, bp: u64) -> PawnStructure {
    let mut isolated = 0;
    let mut doubled = 0;
    let mut passed = 0;

    const FILE_A: u64 = 0x0101010101010101;
    for i in 0..8i8 {
        let wcurrent = (wp >> i) & FILE_A;
        let bcurrent = (bp >> i) & FILE_A;
        let wleft;
        let bleft;
        let wright;
        let bright;
        if i == 0 {
            wleft = 0;
            bleft = 0;
        } else {
            wleft = (wp >> (i - 1)) & FILE_A;
            bleft = (bp >> (i - 1)) & FILE_A;
        }
        if i == 7 {
            wright = 0;
            bright = 0;
        } else {
            wright = (wp >> (i + 1)) & FILE_A;
            bright = (bp >> (i + 1)) & FILE_A;
        }

        doubled -= max(0, wcurrent.count_ones() as i32 - 1);
        doubled += max(0, bcurrent.count_ones() as i32 - 1);

        if wcurrent > 0 && wleft == 0 && wright == 0 {
            isolated -= ISOLATED_PAWN_PENALTY * wcurrent.count_ones() as i32;
        }
        if bcurrent > 0 && bleft == 0 && bright == 0 {
            isolated += ISOLATED_PAWN_PENALTY * bcurrent.count_ones() as i32;
        }

        let mut is_passed = false;
        for rank in 1..7 {
            let p = (wcurrent >> (rank * 8)) & 1;
            if p == 0 {
                continue;
            }

            if is_passed {
                passed += PASSED_PAWN_SCORE[rank - 1];
            } else if bleft >> (rank * 8) == 0
                && bcurrent >> (rank * 8) == 0
                && bright >> (rank * 8) == 0
            {
                is_passed = true;
                passed += PASSED_PAWN_SCORE[rank - 1];
            }
        }
        let mut is_passed = false;
        for rank in 1..7 {
            let p = (bcurrent >> ((7 - rank) * 8)) & 1;
            if p == 0 {
                continue;
            }

            if is_passed {
                passed -= PASSED_PAWN_SCORE[6 - rank];
            } else if wleft << (rank * 8) == 0
                && wcurrent << (rank * 8) == 0
                && wright << (rank * 8) == 0
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
