//! Silnik gry - port engine.c na Rusta (bez zewnetrznych bibliotek).

/// Prosty generator liczb losowych (xorshift64).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1) // stan nie moze byc zerem
    }

    fn nastepna(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Losuje liczbe z przedzialu [min, max] (kolejnosc argumentow dowolna).
    pub fn losuj_liczbe(&mut self, min: i32, max: i32) -> i32 {
        let (min, max) = if min > max { (max, min) } else { (min, max) };
        let zakres = (max - min + 1) as u64;
        min + (self.nastepna() % zakres) as i32
    }
}

pub struct WynikKatastrofy {
    pub hp: i32,
    pub obrazenia: i32,
    pub zniszczony: bool,
}

pub fn oblicz_dochod(dochod: i32) -> i32 {
    dochod.max(0)
}

/// Dochod budynku zalezy od jego zdrowia (hp 0..100).
#[cfg(feature = "asm")]
unsafe extern "C" {
    fn coc_asm_income_hp(dochod: i32, hp: i32) -> i32;
}

pub fn oblicz_dochod_hp(dochod: i32, hp: i32) -> i32 {
    #[cfg(feature = "asm")]
    {
        // The NASM routine implements the same clamped income formula.
        return unsafe { coc_asm_income_hp(dochod, hp) };
    }
    #[cfg(not(feature = "asm"))]
    {
        if dochod <= 0 || hp <= 0 {
            return 0;
        }
        (((dochod as i64) * (hp.min(100) as i64) + 50) / 100)
            .clamp(0, i32::MAX as i64) as i32
    }
}

pub fn wykonaj_katastrofe(hp: i32, obrazenia: i32) -> WynikKatastrofy {
    let obrazenia = obrazenia.max(0);
    let nowe_hp = (hp - obrazenia).max(0);
    WynikKatastrofy {
        hp: nowe_hp,
        obrazenia,
        zniszczony: nowe_hp == 0,
    }
}

pub fn oblicz_nowe_hp(hp: i32, leczenie: i32) -> i32 {
    (hp + leczenie).clamp(0, 100)
}
