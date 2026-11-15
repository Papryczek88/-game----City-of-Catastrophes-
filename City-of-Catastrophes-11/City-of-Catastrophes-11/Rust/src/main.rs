//! City of Catastrophes 11.0 - wersja Rust (tylko biblioteka standardowa).
//! Cale sterowanie: strzalki (lub W/A/S/D) + Enter + Esc.
//!
//! Nowosci w 11.0:
//! - jezyki: angielski (domyslny), polski, niemiecki, portugalski (Portugalia) - moduł `lang`,
//! - menu ustawien (jezyk, podpowiedzi) zapisywane w pliku settings.txt,
//! - ulepszone menu (opisy pozycji, wyszarzone niedostepne opcje),
//! - mapa 6 x 6,
//! - nowe budynki: Solar panel, Coal power plant, Wind turbine,
//! - wszystkie budynki zuzywaja prad (brak pradu = nizszy dochod).

mod engine;
mod lang;

use engine::Rng;
use lang::{fill, tr, Jezyk, JEZYKI, K};
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::{
    io::Read,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

const RESET: &str = "\x1b[0m";
const CYAN: &str = "\x1b[36m";
const ZIEL: &str = "\x1b[32m";
const ZOLTY: &str = "\x1b[33m";
const CZERW: &str = "\x1b[31m";
const SZARY: &str = "\x1b[90m";
const BOLD: &str = "\x1b[1m";
const REV: &str = "\x1b[44;97m"; // ciemnoniebieskie tlo, bialy tekst
const EOL: &str = "\x1b[K\n";

const SZER: usize = 6;
const POLA: usize = SZER * SZER;
const STARTING_LAND: usize = 4; // blok 2 x 2 na srodku mapy
const LAND_PRICE_BASE: i32 = 100;
const LAND_PRICE_STEP: i32 = 25;
const LOAN_AMOUNT: i32 = 500;
const LOAN_INTEREST: i32 = 10;
const CENA_TARCZY: i32 = 200;
const CENA_NAPRAWY: i32 = 50;
const LECZENIE: i32 = 25;
const SZER_LEWEJ: usize = 30; // szerokosc kolumny z mapa (2 + SZER * 4 + odstep)
const SZER_LEGENDY: usize = 76;
const PLIK_USTAWIEN: &str = "settings.txt";

/// Typ budynku. `prad` < 0 = zuzycie pradu, `prad` > 0 = produkcja pradu.
/// Dla turbiny wiatrowej (`wiatr`) `prad` to maksimum - kazdego dnia losowane jest 0..=prad.
struct Typ {
    nazwy: [&'static str; 4], // kolejnosc: EN, PL, DE, PT
    znak: char,
    cena: i32,
    mieszkancy: i32,
    dochod: i32,
    atrakcja: i32,
    czas_budowy: i32,
    fabryka: bool,
    prad: i32,
    wiatr: bool,
}

impl Typ {
    fn nazwa(&self, j: Jezyk) -> &'static str {
        self.nazwy[j.idx()]
    }
}

static TYPY: [Typ; 10] = [
    Typ { nazwy: ["House", "Dom", "Haus", "Casa"],                                         znak: 'H', cena: 50,  mieszkancy: 5,  dochod: 0,  atrakcja: 0,   czas_budowy: 1, fabryka: false, prad: -2,  wiatr: false },
    Typ { nazwy: ["Apartment block", "Blok mieszkalny", "Wohnblock", "Prédio"],            znak: 'B', cena: 120, mieszkancy: 20, dochod: 1,  atrakcja: 0,   czas_budowy: 3, fabryka: false, prad: -6,  wiatr: false },
    Typ { nazwy: ["Luxury apartment", "Luksusowy apartament", "Luxusapartment", "Apartamento de luxo"], znak: 'A', cena: 200, mieszkancy: 40, dochod: 2,  atrakcja: 10,  czas_budowy: 5, fabryka: false, prad: -12, wiatr: false },
    Typ { nazwy: ["Park", "Park", "Park", "Parque"],                                       znak: 'P', cena: 80,  mieszkancy: 0,  dochod: 1,  atrakcja: 15,  czas_budowy: 2, fabryka: false, prad: -1,  wiatr: false },
    Typ { nazwy: ["Restaurant", "Restauracja", "Restaurant", "Restaurante"],               znak: 'R', cena: 150, mieszkancy: 5,  dochod: 5,  atrakcja: 20,  czas_budowy: 3, fabryka: false, prad: -8,  wiatr: false },
    Typ { nazwy: ["Farm", "Farma", "Bauernhof", "Quinta"],                                 znak: 'F', cena: 60,  mieszkancy: 0,  dochod: 1,  atrakcja: 0,   czas_budowy: 2, fabryka: false, prad: -3,  wiatr: false },
    Typ { nazwy: ["Factory", "Fabryka", "Fabrik", "Fábrica"],                              znak: 'I', cena: 300, mieszkancy: 10, dochod: 10, atrakcja: -5,  czas_budowy: 7, fabryka: true,  prad: -25, wiatr: false },
    Typ { nazwy: ["Solar panel", "Panel słoneczny", "Solarpanel", "Painel solar"],         znak: 'S', cena: 50,  mieszkancy: 0,  dochod: 0,  atrakcja: 0,   czas_budowy: 2, fabryka: false, prad: 10,  wiatr: false },
    Typ { nazwy: ["Coal power plant", "Elektrownia węglowa", "Kohlekraftwerk", "Central a carvão"], znak: 'C', cena: 200, mieszkancy: 0,  dochod: 0,  atrakcja: -10, czas_budowy: 4, fabryka: false, prad: 50,  wiatr: false },
    Typ { nazwy: ["Wind turbine", "Turbina wiatrowa", "Windturbine", "Turbina eólica"],    znak: 'W', cena: 10,  mieszkancy: 0,  dochod: 0,  atrakcja: 0,   czas_budowy: 1, fabryka: false, prad: 10,  wiatr: true },
];

const MENU: [K; 10] = [K::Menu0, K::Menu1, K::Menu2, K::Menu3, K::Menu4, K::Menu5, K::Menu6, K::Menu7, K::Menu8, K::Menu9];
const OPISY: [K; 10] = [K::Desc0, K::Desc1, K::Desc2, K::Desc3, K::Desc4, K::Desc5, K::Desc6, K::Desc7, K::Desc8, K::Desc9];
const KATASTROFY: [K; 6] = [K::Dis0, K::Dis1, K::Dis2, K::Dis3, K::Dis4, K::Dis5];

#[derive(Clone, Copy, PartialEq)]
enum Stan {
    Puste,
    Budowa,
    Gotowe,
}

#[derive(Clone, Copy, PartialEq)]
enum Tryb {
    Budowa,
    Naprawa,
}

#[derive(Clone, Copy)]
enum Klawisz {
    Gora,
    Dol,
    Lewo,
    Prawo,
    Enter,
    Esc,
}

#[derive(Clone, Copy)]
struct Pole {
    stan: Stan,
    typ: usize,
    hp: i32,
    dni: i32,
    moc: i32, // ostatnio wylosowana moc turbiny wiatrowej
}

const PUSTE: Pole = Pole { stan: Stan::Puste, typ: 0, hp: 0, dni: 0, moc: 0 };

struct Gra {
    mapa: [Pole; POLA],
    dzialki: [bool; POLA],
    pieniadze: i32,
    ludzie: i32,
    atrakcja: i32,
    punkty: i32,
    dzien: i32,
    tarcze: i32,
    komunikat: String,
    rng: Rng,
    kredyt: i32,
    do_splaty: i32,
    jezyk: Jezyk,
    podpowiedzi: bool,
}

impl Gra {
    fn nowa(seed: u64) -> Gra {
        // poczatkowe dzialki: blok 2 x 2 na srodku mapy
        let mut dzialki = [false; POLA];
        let start = SZER / 2 - 1;
        for y in start..start + 2 {
            for x in start..start + 2 {
                dzialki[y * SZER + x] = true;
            }
        }

        Gra {
            mapa: [PUSTE; POLA],
            dzialki,
            pieniadze: 500,
            ludzie: 0,
            atrakcja: 0,
            punkty: 0,
            dzien: 1,
            tarcze: 0,
            komunikat: String::new(),
            rng: Rng::new(seed),
            kredyt: 0,
            do_splaty: 0,
            jezyk: Jezyk::En,
            podpowiedzi: true,
        }
    }

    fn liczba_dzialek(&self) -> usize {
        self.dzialki.iter().filter(|&&w| w).count()
    }

    fn cena_dzialki(&self) -> i32 {
        let kupione = self.liczba_dzialek() as i32 - STARTING_LAND as i32;
        LAND_PRICE_BASE + kupione.max(0) * LAND_PRICE_STEP
    }

    fn budowa_trwa(&self) -> bool {
        self.mapa.iter().any(|p| p.stan == Stan::Budowa)
    }

    fn dodaj_komunikat(&mut self, tekst: &str) {
        if tekst.is_empty() {
            return;
        }
        if !self.komunikat.is_empty() {
            self.komunikat.push(' ');
        }
        self.komunikat.push_str(tekst);
    }
}

// ---------------------------------------------------------------- ustawienia

fn wczytaj_ustawienia() -> Option<(Jezyk, bool)> {
    let tekst = std::fs::read_to_string(PLIK_USTAWIEN).ok()?;
    let mut jezyk = Jezyk::En;
    let mut podpowiedzi = true;
    let mut znaleziono = false;
    for linia in tekst.lines() {
        if let Some((klucz, wartosc)) = linia.split_once('=') {
            let wartosc = wartosc.trim();
            match klucz.trim() {
                "lang" => {
                    if let Some(j) = Jezyk::z_kodu(wartosc) {
                        jezyk = j;
                        znaleziono = true;
                    }
                }
                "hints" => {
                    podpowiedzi = wartosc != "0";
                }
                _ => {}
            }
        }
    }
    if znaleziono {
        Some((jezyk, podpowiedzi))
    } else {
        None
    }
}

fn zapisz_ustawienia(g: &Gra) {
    let tresc = format!(
        "lang={}\nhints={}\n",
        g.jezyk.kod(),
        if g.podpowiedzi { 1 } else { 0 }
    );
    let _ = std::fs::write(PLIK_USTAWIEN, tresc); // brak zapisu nie jest bledem krytycznym
}

// ---------------------------------------------------------------- terminal

fn z_znaku(c: u8) -> Option<Klawisz> {
    match c {
        b'w' | b'W' => Some(Klawisz::Gora),
        b's' | b'S' => Some(Klawisz::Dol),
        b'a' | b'A' => Some(Klawisz::Lewo),
        b'd' | b'D' => Some(Klawisz::Prawo),
        b'\r' | b'\n' | b' ' => Some(Klawisz::Enter),
        3 => Some(Klawisz::Esc), // Ctrl+C
        _ => None,
    }
}

struct Terminal {
    #[cfg(unix)]
    stare: Option<String>,
    #[cfg(unix)]
    rx: Receiver<u8>,
}

#[cfg(unix)]
impl Terminal {
    fn new() -> Terminal {
        // zapamietaj ustawienia terminala i wlacz tryb "surowy" (bez Entera, bez echa)
        let stare = Command::new("stty")
            .arg("-g")
            .stdin(Stdio::inherit())
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
        if stare.is_some() {
            let _ = Command::new("stty")
                .args(["-icanon", "-echo", "-isig", "min", "1", "time", "0"])
                .stdin(Stdio::inherit())
                .status();
        }

        // watek czytajacy bajty - dzieki niemu mozna czekac z limitem czasu na sekwencje strzalek
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut stdin = io::stdin();
            let mut b = [0u8; 1];
            while let Ok(1) = stdin.read(&mut b) {
                if tx.send(b[0]).is_err() {
                    break;
                }
            }
        });

        print!("\x1b[?1049h\x1b[?25l\x1b[2J");
        let _ = io::stdout().flush();
        Terminal { stare, rx }
    }

    fn przywroc(&self) {
        print!("{}\x1b[?25h\x1b[?1049l", RESET);
        let _ = io::stdout().flush();
        if let Some(s) = &self.stare {
            let _ = Command::new("stty").arg(s).stdin(Stdio::inherit()).status();
        }
    }

    fn koniec(&self) -> ! {
        self.przywroc();
        std::process::exit(0)
    }

    fn klawisz(&self) -> Klawisz {
        let limit = Duration::from_millis(60);
        loop {
            let c = match self.rx.recv() {
                Ok(c) => c,
                Err(_) => self.koniec(), // koniec wejscia
            };
            if c != 27 {
                if let Some(k) = z_znaku(c) {
                    return k;
                }
                continue;
            }
            // Esc albo poczatek sekwencji strzalki: ESC [ A
            let a = match self.rx.recv_timeout(limit) {
                Ok(a) => a,
                Err(_) => return Klawisz::Esc,
            };
            if a != b'[' && a != b'O' {
                return Klawisz::Esc;
            }
            let mut b = match self.rx.recv_timeout(limit) {
                Ok(b) => b,
                Err(_) => return Klawisz::Esc,
            };
            while b.is_ascii_digit() || b == b';' {
                b = match self.rx.recv_timeout(limit) {
                    Ok(b) => b,
                    Err(_) => return Klawisz::Esc,
                };
            }
            match b {
                b'A' => return Klawisz::Gora,
                b'B' => return Klawisz::Dol,
                b'D' => return Klawisz::Lewo,
                b'C' => return Klawisz::Prawo,
                _ => {}
            }
        }
    }
}

#[cfg(windows)]
extern "C" {
    fn _getch() -> i32;
}

#[cfg(windows)]
extern "system" {
    fn GetStdHandle(n: u32) -> *mut std::ffi::c_void;
    fn GetConsoleMode(h: *mut std::ffi::c_void, tryb: *mut u32) -> i32;
    fn SetConsoleMode(h: *mut std::ffi::c_void, tryb: u32) -> i32;
    fn SetConsoleOutputCP(strona: u32) -> i32;
}

#[cfg(windows)]
impl Terminal {
    fn new() -> Terminal {
        unsafe {
            SetConsoleOutputCP(65001); // UTF-8: polskie i inne znaki diakrytyczne
            let h = GetStdHandle(0xFFFF_FFF5); // STD_OUTPUT_HANDLE
            let mut tryb: u32 = 0;
            if !h.is_null() && GetConsoleMode(h, &mut tryb) != 0 {
                SetConsoleMode(h, tryb | 0x0004); // ENABLE_VIRTUAL_TERMINAL_PROCESSING
            }
        }
        print!("\x1b[?1049h\x1b[?25l\x1b[2J");
        let _ = io::stdout().flush();
        Terminal {}
    }

    fn przywroc(&self) {
        print!("{}\x1b[?25h\x1b[?1049l", RESET);
        let _ = io::stdout().flush();
    }

    fn klawisz(&self) -> Klawisz {
        loop {
            let mut z = unsafe { _getch() };
            if z == 0 || z == 224 {
                z = unsafe { _getch() };
                if z == 72 { return Klawisz::Gora; }
                if z == 80 { return Klawisz::Dol; }
                if z == 75 { return Klawisz::Lewo; }
                if z == 77 { return Klawisz::Prawo; }
                continue;
            }
            if z == 27 {
                return Klawisz::Esc;
            }
            if let Some(k) = z_znaku(z as u8) {
                return k;
            }
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.przywroc(); // dziala tez przy panice
    }
}

// ---------------------------------------------------------------- rysowanie

fn ln(bufor: &mut String, tekst: &str) {
    bufor.push_str(tekst);
    bufor.push_str(EOL);
}

fn kol(kolor: &str, tekst: &str) -> String {
    format!("{}{}{}", kolor, tekst, RESET)
}

fn hint(tekst: &str) -> String {
    format!("  {}{}{}", CYAN, tekst, RESET)
}

fn kolor_hp(hp: i32) -> &'static str {
    if hp > 60 {
        ZIEL
    } else if hp > 30 {
        ZOLTY
    } else {
        CZERW
    }
}

fn pasek_hp(hp: i32) -> String {
    let n = (((hp + 9) / 10).clamp(0, 10)) as usize;
    kol(
        kolor_hp(hp),
        &format!("[{}{}] {} HP", "#".repeat(n), "-".repeat(10 - n), hp),
    )
}

fn wiersz_mapy(g: &Gra, y: usize, kursor: Option<usize>) -> String {
    let mut s = String::from("  ");
    for x in 0..SZER {
        let i = y * SZER + x;
        let p = &g.mapa[i];
        let (k, znak) = match p.stan {
            Stan::Puste => {
                if g.dzialki[i] {
                    (RESET, ' ')
                } else {
                    (CZERW, '·')
                }
            }
            Stan::Budowa => (ZOLTY, '#'),
            Stan::Gotowe => (kolor_hp(p.hp), TYPY[p.typ].znak),
        };
        if kursor == Some(i) {
            s.push_str(&format!("{}[{}]{} ", REV, znak, RESET));
        } else {
            s.push_str(&format!("[{}{}{}] ", k, znak, RESET));
        }
    }
    s
}

/// Lamie liste wpisow na linie o maksymalnej szerokosci `szer` znakow.
fn zawin(wpisy: &[String], szer: usize) -> Vec<String> {
    let mut linie: Vec<String> = Vec::new();
    let mut biezaca = String::new();
    for w in wpisy {
        let dl = biezaca.chars().count() + w.chars().count() + 2;
        if !biezaca.is_empty() && dl > szer {
            linie.push(biezaca);
            biezaca = String::new();
        }
        if !biezaca.is_empty() {
            biezaca.push_str("  ");
        }
        biezaca.push_str(w);
    }
    if !biezaca.is_empty() {
        linie.push(biezaca);
    }
    linie
}

/// Bilans pradu: (produkcja, zapotrzebowanie) wszystkich ukonczonych budynkow.
fn bilans(g: &Gra) -> (i32, i32) {
    let mut produkcja = 0;
    let mut potrzeba = 0;
    for p in g.mapa.iter() {
        if p.stan != Stan::Gotowe {
            continue;
        }
        let t = &TYPY[p.typ];
        if t.prad > 0 {
            let baza = if t.wiatr { p.moc } else { t.prad };
            produkcja += engine::oblicz_dochod_hp(baza, p.hp);
        } else {
            potrzeba += -t.prad;
        }
    }
    (produkcja, potrzeba)
}

/// Opis pradu budynku do panelu informacyjnego, np. "-8", "+10", "+4 (0-10)".
fn opis_pradu(t: &Typ, p: &Pole) -> String {
    if t.wiatr {
        format!("+{} (0-{})", p.moc, t.prad)
    } else if t.prad > 0 {
        format!("+{}", engine::oblicz_dochod_hp(t.prad, p.hp))
    } else {
        format!("{}", t.prad)
    }
}

/// Caly ekran: naglowek, mapa po lewej, kolumna "prawa" obok, linie "dol" pod spodem.
fn ekran(g: &Gra, kursor: Option<usize>, prawa: &[String], dol: &[String]) {
    let j = g.jezyk;
    let mut s = String::from("\x1b[H");
    ln(&mut s, &format!("{}{}  ===== CITY OF CATASTROPHES 11.0 ====={}", BOLD, CYAN, RESET));

    let (produkcja, potrzeba) = bilans(g);
    let kolor_pradu = if produkcja >= potrzeba { ZIEL } else { CZERW };
    ln(
        &mut s,
        &format!(
            "  {}: {}   {}: {}   {}: {}   {}: {}   {}: {}",
            tr(j, K::Day),
            kol(ZOLTY, &g.dzien.to_string()),
            tr(j, K::Money),
            kol(ZIEL, &g.pieniadze.to_string()),
            tr(j, K::Residents),
            g.ludzie,
            tr(j, K::Loan),
            g.do_splaty,
            tr(j, K::Power),
            kol(kolor_pradu, &format!("{}/{}", produkcja, potrzeba)),
        ),
    );
    ln(
        &mut s,
        &format!(
            "  {}: {}   {}: {}   {}: {}   {}: {}/{}",
            tr(j, K::Attractiveness),
            g.atrakcja,
            tr(j, K::Shields),
            g.tarcze,
            tr(j, K::Points),
            g.punkty,
            tr(j, K::Land),
            g.liczba_dzialek(),
            POLA
        ),
    );
    ln(&mut s, "");

    let wiersze = prawa.len().max(SZER + 1);
    for r in 0..wiersze {
        let lewa = if r == 0 {
            let tytul = tr(j, K::MapTitle);
            let odstep = SZER_LEWEJ.saturating_sub(2 + tytul.chars().count());
            format!("{}  {}{}{}", BOLD, tytul, RESET, " ".repeat(odstep))
        } else if r <= SZER {
            format!(
                "{}{}",
                wiersz_mapy(g, r - 1, kursor),
                " ".repeat(SZER_LEWEJ - 2 - SZER * 4)
            )
        } else {
            " ".repeat(SZER_LEWEJ)
        };
        let p = prawa.get(r).map(String::as_str).unwrap_or("");
        ln(&mut s, &format!("{}{}", lewa, p));
    }

    // legenda budowana z listy budynkow (zawsze zgodna z tlumaczeniem)
    let mut wpisy: Vec<String> = TYPY
        .iter()
        .map(|t| format!("{} {}", t.znak, t.nazwa(j)))
        .collect();
    wpisy.push(format!("# {}", tr(j, K::UnderConstruction)));
    wpisy.push(format!("· {}", tr(j, K::UnownedLand)));
    for linia in zawin(&wpisy, SZER_LEGENDY) {
        ln(&mut s, &format!("  {}", linia));
    }

    let dni = tr(j, K::DaysShort);
    let budowy: Vec<String> = g
        .mapa
        .iter()
        .filter(|p| p.stan == Stan::Budowa)
        .map(|p| format!("{} ({}{})", TYPY[p.typ].nazwa(j), p.dni, dni))
        .collect();
    let budowy = if budowy.is_empty() {
        tr(j, K::NoneWord).to_string()
    } else {
        budowy.join(", ")
    };
    ln(&mut s, &format!("  {}: {}", tr(j, K::UnderConstruction), budowy));

    // podpowiedzi (kolor CYAN) mozna wylaczyc w ustawieniach
    let znacznik = format!("  {}", CYAN);
    for linia in dol {
        if !g.podpowiedzi && linia.starts_with(&znacznik) {
            continue;
        }
        ln(&mut s, linia);
    }
    s.push_str("\x1b[J");

    let mut wyj = io::stdout().lock();
    let _ = wyj.write_all(s.as_bytes());
    let _ = wyj.flush();
}

// ---------------------------------------------------------------- logika

fn przejdz_dni(g: &mut Gra, liczba_dni: i32) {
    let mut min_proc = 100;
    for _ in 0..liczba_dni {
        g.dzien += 1;

        // wiatr: kazdego dnia nowa losowa moc turbin (0..=max)
        for i in 0..POLA {
            if g.mapa[i].stan == Stan::Gotowe && TYPY[g.mapa[i].typ].wiatr {
                let max = TYPY[g.mapa[i].typ].prad;
                g.mapa[i].moc = g.rng.losuj_liczbe(0, max);
            }
        }

        // prad: przy niedoborze dochod wszystkich budynkow spada proporcjonalnie
        let (produkcja, potrzeba) = bilans(g);
        let procent: i32 = if potrzeba <= 0 || produkcja >= potrzeba {
            100
        } else {
            ((produkcja as i64) * 100 / (potrzeba as i64)) as i32
        };
        if procent < min_proc {
            min_proc = procent;
        }

        for i in 0..POLA {
            match g.mapa[i].stan {
                Stan::Gotowe => {
                    let p = g.mapa[i];
                    let dochod = engine::oblicz_dochod_hp(
                        engine::oblicz_dochod(TYPY[p.typ].dochod),
                        p.hp,
                    );
                    g.pieniadze += (((dochod as i64) * (procent as i64) + 50) / 100) as i32;
                }
                Stan::Budowa => {
                    g.mapa[i].dni -= 1;
                    if g.mapa[i].dni <= 0 {
                        g.mapa[i].stan = Stan::Gotowe;
                        let t = &TYPY[g.mapa[i].typ];
                        if t.wiatr {
                            g.mapa[i].moc = g.rng.losuj_liczbe(0, t.prad);
                        }
                        g.ludzie += t.mieszkancy;
                        g.atrakcja += t.atrakcja;
                        g.punkty += 10;
                        let m = fill(tr(g.jezyk, K::Completed), &[&t.nazwa(g.jezyk)]);
                        g.dodaj_komunikat(&m);
                    }
                }
                Stan::Puste => {}
            }
        }
    }

    if min_proc < 100 {
        let m = fill(tr(g.jezyk, K::Blackout), &[&min_proc]);
        g.dodaj_komunikat(&m);
    }
}

fn katastrofa(g: &mut Gra) {
    const DNI: [i32; 6] = [1, 3, 5, 2, 4, 4];

    let jest_fabryka = g
        .mapa
        .iter()
        .any(|p| p.stan == Stan::Gotowe && TYPY[p.typ].fabryka);

    let los: usize = if jest_fabryka && g.rng.losuj_liczbe(0, 1) == 0 {
        5
    } else {
        g.rng.losuj_liczbe(0, 4) as usize
    };
    let mut obrazenia = if los == 5 { 70 } else { g.rng.losuj_liczbe(20, 59) };

    if g.tarcze > 0 {
        obrazenia /= 2;
        g.tarcze -= 1;
        let m = tr(g.jezyk, K::ShieldHalved);
        g.dodaj_komunikat(m);
    }
    let m = fill(tr(g.jezyk, K::DisasterDamage), &[&tr(g.jezyk, KATASTROFY[los]), &obrazenia]);
    g.dodaj_komunikat(&m);

    for i in 0..POLA {
        if g.mapa[i].stan != Stan::Gotowe {
            continue;
        }
        let w = engine::wykonaj_katastrofe(g.mapa[i].hp, obrazenia);
        g.mapa[i].hp = w.hp;
        if w.zniszczony {
            let t = &TYPY[g.mapa[i].typ];
            let m = fill(tr(g.jezyk, K::Destroyed), &[&t.nazwa(g.jezyk)]);
            g.dodaj_komunikat(&m);
            g.ludzie -= t.mieszkancy;
            g.atrakcja -= t.atrakcja;
            g.mapa[i] = PUSTE;
        }
    }
    g.ludzie = g.ludzie.max(0);
    g.atrakcja = g.atrakcja.max(0);

    przejdz_dni(g, DNI[los]);
    let m = fill(tr(g.jezyk, K::DisasterCost), &[&DNI[los]]);
    g.dodaj_komunikat(&m);
    g.punkty += 30;
}

// ---------------------------------------------------------------- menu

/// Ktore pozycje menu glownego sa w tej chwili dostepne (niedostepne sa wyszarzone).
fn dostepne_menu(g: &Gra) -> [bool; 10] {
    let wolne_pole = (0..POLA).any(|i| sprawdz_pole(g, i, Tryb::Budowa).is_none());
    let wolna_ziemia = (0..POLA).any(|i| !g.dzialki[i]);
    let uszkodzone = (0..POLA).any(|i| sprawdz_pole(g, i, Tryb::Naprawa).is_none());
    [
        wolne_pole,
        wolna_ziemia && g.pieniadze >= g.cena_dzialki(),
        g.kredyt <= 0,
        g.kredyt > 0 && g.pieniadze >= g.do_splaty,
        g.pieniadze >= CENA_TARCZY,
        uszkodzone && g.pieniadze >= CENA_NAPRAWY,
        true,
        true,
        true,
        true,
    ]
}

fn etykieta_menu(j: Jezyk, i: usize) -> String {
    match i {
        2 => fill(tr(j, MENU[i]), &[&LOAN_AMOUNT]),
        4 => fill(tr(j, MENU[i]), &[&CENA_TARCZY]),
        5 => fill(tr(j, MENU[i]), &[&CENA_NAPRAWY]),
        _ => tr(j, MENU[i]).to_string(),
    }
}

fn opis_menu(j: Jezyk, i: usize) -> String {
    match i {
        2 => {
            let zwrot = LOAN_AMOUNT * (100 + LOAN_INTEREST) / 100;
            fill(tr(j, OPISY[i]), &[&LOAN_AMOUNT, &zwrot, &LOAN_INTEREST])
        }
        5 => fill(tr(j, OPISY[i]), &[&LECZENIE]),
        _ => tr(j, OPISY[i]).to_string(),
    }
}

fn menu_glowne(t: &Terminal, g: &Gra, wybrane: &mut usize) -> usize {
    let j = g.jezyk;
    let n = MENU.len();

    loop {
        let dostepne = dostepne_menu(g);
        let mut prawa = vec![kol(BOLD, tr(j, K::MenuTitle))];
        for i in 0..n {
            let nazwa = etykieta_menu(j, i);
            if i == *wybrane {
                prawa.push(format!("{}> {:<30}{}", REV, nazwa, RESET));
            } else if dostepne[i] {
                prawa.push(format!("  {:<30}", nazwa));
            } else {
                prawa.push(format!("{}  {:<30}{}", SZARY, nazwa, RESET));
            }
        }
        let mut dol = vec![String::new()];
        if !g.komunikat.is_empty() {
            dol.push(format!("  {}", kol(ZOLTY, &g.komunikat)));
        }
        dol.push(hint(&opis_menu(j, *wybrane)));
        dol.push(hint(tr(j, K::HintMenu)));
        ekran(g, None, &prawa, &dol);

        match t.klawisz() {
            Klawisz::Gora => *wybrane = (*wybrane + n - 1) % n,
            Klawisz::Dol => *wybrane = (*wybrane + 1) % n,
            Klawisz::Enter => return *wybrane,
            Klawisz::Esc => return n - 1,
            _ => {}
        }
    }
}

fn potwierdz(t: &Terminal, g: &Gra, pytanie: &str) -> bool {
    let j = g.jezyk;
    let mut tak = false;
    loop {
        let opcje = if tak {
            format!("  {}    {} > {} {}", tr(j, K::No), REV, tr(j, K::Yes), RESET)
        } else {
            format!("  {} > {} {}    {}", REV, tr(j, K::No), RESET, tr(j, K::Yes))
        };
        let prawa = vec![kol(BOLD, tr(j, K::ConfirmTitle))];
        let dol = vec![
            String::new(),
            format!("  {}", kol(ZOLTY, pytanie)),
            opcje,
            String::new(),
            hint(tr(j, K::HintConfirm)),
        ];
        ekran(g, None, &prawa, &dol);

        match t.klawisz() {
            Klawisz::Lewo | Klawisz::Prawo | Klawisz::Gora | Klawisz::Dol => tak = !tak,
            Klawisz::Enter => return tak,
            Klawisz::Esc => return false,
        }
    }
}

fn wybierz_budynek(t: &Terminal, g: &Gra) -> Option<usize> {
    let j = g.jezyk;
    let mut wybrane = 0usize;
    let mut blad = String::new();
    loop {
        let mut prawa = vec![kol(BOLD, tr(j, K::ShopTitle))];
        prawa.push(kol(
            CYAN,
            &format!(
                "  {:<22}{:<6}{:<5}{}",
                tr(j, K::ColName),
                tr(j, K::ColPrice),
                tr(j, K::ColDays),
                tr(j, K::ColPower)
            ),
        ));
        for (i, ty) in TYPY.iter().enumerate() {
            let prad = if ty.wiatr {
                format!("0-{}", ty.prad)
            } else {
                format!("{:+}", ty.prad)
            };
            let tekst = format!(
                "{}{:<22}{:<6}{:<5}{}",
                if i == wybrane { "> " } else { "  " },
                ty.nazwa(j),
                ty.cena,
                ty.czas_budowy,
                prad
            );
            let przod = if i == wybrane {
                REV
            } else if ty.cena > g.pieniadze {
                CZERW
            } else {
                ""
            };
            prawa.push(format!("{}{}{}", przod, tekst, RESET));
        }
        let ty = &TYPY[wybrane];
        let plus = if ty.atrakcja > 0 { "+" } else { "" };
        let atrakcja = format!("{}{}", plus, ty.atrakcja);
        let linia_pradu = if ty.wiatr {
            fill(tr(j, K::PowerMakesRandom), &[&ty.prad])
        } else if ty.prad > 0 {
            fill(tr(j, K::PowerMakes), &[&ty.prad])
        } else {
            fill(tr(j, K::PowerNeeds), &[&(-ty.prad)])
        };
        let dol = vec![
            fill(tr(j, K::ShopInfo), &[&ty.mieszkancy, &ty.dochod, &atrakcja]),
            linia_pradu,
            format!("  {}", kol(CZERW, &blad)),
            hint(tr(j, K::HintShop)),
        ];
        ekran(g, None, &prawa, &dol);

        blad.clear();
        match t.klawisz() {
            Klawisz::Gora => wybrane = (wybrane + TYPY.len() - 1) % TYPY.len(),
            Klawisz::Dol => wybrane = (wybrane + 1) % TYPY.len(),
            Klawisz::Esc => return None,
            Klawisz::Enter => {
                if TYPY[wybrane].cena > g.pieniadze {
                    blad = tr(j, K::NotEnoughMoney).to_string();
                } else {
                    return Some(wybrane);
                }
            }
            _ => {}
        }
    }
}

fn sprawdz_dzialke(g: &Gra, i: usize) -> Option<&'static str> {
    if g.dzialki[i] {
        Some(tr(g.jezyk, K::LandOwned))
    } else {
        None
    }
}

fn wybierz_dzialke(t: &Terminal, g: &Gra) -> Option<usize> {
    let j = g.jezyk;
    let mut kursor = (0..POLA).find(|&i| !g.dzialki[i]).unwrap_or(0);
    let mut blad = String::new();

    loop {
        let mut prawa = vec![kol(BOLD, tr(j, K::LandTitle))];
        prawa.push(fill(tr(j, K::PriceLabel), &[&g.cena_dzialki()]));
        prawa.push(fill(
            tr(j, K::FreePlots),
            &[&(0..POLA).filter(|&i| !g.dzialki[i]).count()],
        ));
        let dol = vec![
            String::new(),
            format!("  {}", kol(CZERW, &blad)),
            hint(tr(j, K::HintLand)),
        ];
        ekran(g, Some(kursor), &prawa, &dol);

        blad.clear();
        match t.klawisz() {
            Klawisz::Gora if kursor >= SZER => kursor -= SZER,
            Klawisz::Dol if kursor < POLA - SZER => kursor += SZER,
            Klawisz::Lewo if kursor % SZER > 0 => kursor -= 1,
            Klawisz::Prawo if kursor % SZER < SZER - 1 => kursor += 1,
            Klawisz::Esc => return None,
            Klawisz::Enter => match sprawdz_dzialke(g, kursor) {
                None => return Some(kursor),
                Some(powod) => blad = powod.to_string(),
            },
            _ => {}
        }
    }
}

/// None = pole poprawne, Some(powod) = odmowa.
fn sprawdz_pole(g: &Gra, i: usize, tryb: Tryb) -> Option<&'static str> {
    let j = g.jezyk;
    let p = &g.mapa[i];
    if tryb == Tryb::Budowa {
        if !g.dzialki[i] {
            return Some(tr(j, K::BuyLandFirst));
        }
        return if p.stan == Stan::Puste { None } else { Some(tr(j, K::Occupied)) };
    }
    if p.stan != Stan::Gotowe {
        return Some(tr(j, K::NoFinished));
    }
    if p.hp >= 100 {
        return Some(tr(j, K::Undamaged));
    }
    None
}

fn wybierz_pole(t: &Terminal, g: &Gra, tryb: Tryb, typ: Option<usize>) -> Option<usize> {
    let j = g.jezyk;
    let mut kursor = (0..POLA)
        .find(|&i| sprawdz_pole(g, i, tryb).is_none())
        .unwrap_or(0);
    let mut blad = String::new();

    loop {
        let p = &g.mapa[kursor];
        let tytul = if tryb == Tryb::Budowa { tr(j, K::ChooseSite) } else { tr(j, K::ChooseRepair) };
        let mut prawa = vec![kol(BOLD, tytul)];
        if let (Tryb::Budowa, Some(ty)) = (tryb, typ) {
            prawa.push(fill(tr(j, K::Placing), &[&TYPY[ty].nazwa(j)]));
        }
        match p.stan {
            Stan::Puste => {
                if g.dzialki[kursor] {
                    prawa.push(tr(j, K::EmptyTile).to_string());
                } else {
                    prawa.push(tr(j, K::UnownedLand).to_string());
                }
            }
            Stan::Budowa => {
                prawa.push(fill(tr(j, K::TileUnderConstr), &[&TYPY[p.typ].nazwa(j)]));
                prawa.push(fill(tr(j, K::DaysRemaining), &[&p.dni]));
            }
            Stan::Gotowe => {
                let ty = &TYPY[p.typ];
                prawa.push(ty.nazwa(j).to_string());
                prawa.push(pasek_hp(p.hp));
                prawa.push(fill(
                    tr(j, K::IncomeLine),
                    &[&engine::oblicz_dochod_hp(ty.dochod, p.hp), &ty.dochod],
                ));
                prawa.push(fill(tr(j, K::ResidentsLine), &[&ty.mieszkancy]));
                prawa.push(fill(tr(j, K::PowerLabel), &[&opis_pradu(ty, p)]));
            }
        }
        let dol = vec![
            String::new(),
            format!("  {}", kol(CZERW, &blad)),
            hint(tr(j, K::HintMove)),
        ];
        ekran(g, Some(kursor), &prawa, &dol);

        blad.clear();
        match t.klawisz() {
            Klawisz::Gora if kursor >= SZER => kursor -= SZER,
            Klawisz::Dol if kursor < POLA - SZER => kursor += SZER,
            Klawisz::Lewo if kursor % SZER > 0 => kursor -= 1,
            Klawisz::Prawo if kursor % SZER < SZER - 1 => kursor += 1,
            Klawisz::Esc => return None,
            Klawisz::Enter => match sprawdz_pole(g, kursor, tryb) {
                None => return Some(kursor),
                Some(powod) => blad = powod.to_string(),
            },
            _ => {}
        }
    }
}

// ---------------------------------------------------------------- ustawienia (menu)

fn wybierz_jezyk(t: &Terminal, g: &mut Gra) {
    let mut wybrane = JEZYKI.iter().position(|&x| x == g.jezyk).unwrap_or(0);
    loop {
        let j = g.jezyk;
        let mut prawa = vec![kol(BOLD, tr(j, K::LangTitle))];
        for (i, l) in JEZYKI.iter().enumerate() {
            let tekst = format!(
                "{}{} {}",
                if i == wybrane { "> " } else { "  " },
                if *l == g.jezyk { "*" } else { " " },
                l.nazwa()
            );
            if i == wybrane {
                prawa.push(format!("{}{:<30}{}", REV, tekst, RESET));
            } else {
                prawa.push(format!("{:<30}", tekst));
            }
        }
        let dol = vec![String::new(), hint(tr(j, K::HintLang))];
        ekran(g, None, &prawa, &dol);

        match t.klawisz() {
            Klawisz::Gora => wybrane = (wybrane + JEZYKI.len() - 1) % JEZYKI.len(),
            Klawisz::Dol => wybrane = (wybrane + 1) % JEZYKI.len(),
            Klawisz::Enter => {
                g.jezyk = JEZYKI[wybrane];
                g.komunikat = tr(g.jezyk, K::LangChanged).to_string();
                zapisz_ustawienia(g);
                return;
            }
            Klawisz::Esc => return,
            _ => {}
        }
    }
}

fn ekran_ustawien(t: &Terminal, g: &mut Gra) {
    let mut wybrane = 0usize;
    let n = 3usize;
    loop {
        let j = g.jezyk;
        let stan = if g.podpowiedzi { tr(j, K::On) } else { tr(j, K::Off) };
        let pozycje = vec![
            fill(tr(j, K::SetLanguage), &[&j.nazwa()]),
            fill(tr(j, K::SetHints), &[&stan]),
            tr(j, K::Back).to_string(),
        ];
        let mut prawa = vec![kol(BOLD, tr(j, K::SettingsTitle))];
        for (i, p) in pozycje.iter().enumerate() {
            if i == wybrane {
                prawa.push(format!("{}> {:<30}{}", REV, p, RESET));
            } else {
                prawa.push(format!("  {:<30}", p));
            }
        }
        let dol = vec![String::new(), hint(tr(j, K::HintSettings))];
        ekran(g, None, &prawa, &dol);

        match t.klawisz() {
            Klawisz::Gora => wybrane = (wybrane + n - 1) % n,
            Klawisz::Dol => wybrane = (wybrane + 1) % n,
            Klawisz::Enter => match wybrane {
                0 => wybierz_jezyk(t, g),
                1 => {
                    g.podpowiedzi = !g.podpowiedzi;
                    zapisz_ustawienia(g);
                }
                _ => return,
            },
            Klawisz::Esc => return,
            _ => {}
        }
    }
}

// ---------------------------------------------------------------- koniec gry

fn podsumowanie(g: &Gra) -> String {
    let j = g.jezyk;
    format!(
        "  {}: {}\n  {}: {}\n  {}: {}\n  {}: {}\n  {}: {}/{}\n  {}: {}\n  {}: {}\n",
        tr(j, K::SumDays),
        g.dzien,
        tr(j, K::Points),
        g.punkty,
        tr(j, K::Residents),
        g.ludzie,
        tr(j, K::Money),
        g.pieniadze,
        tr(j, K::SumOwnedLand),
        g.liczba_dzialek(),
        POLA,
        tr(j, K::SumLoanDue),
        g.do_splaty,
        tr(j, K::Attractiveness),
        g.atrakcja
    )
}

fn main() {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(12345);
    let mut g = Gra::nowa(seed);
    let zapisane = wczytaj_ustawienia();
    if let Some((j, p)) = zapisane {
        g.jezyk = j;
        g.podpowiedzi = p;
    }
    let term = Terminal::new();
    if zapisane.is_none() {
        // pierwsze uruchomienie: najpierw wybor jezyka
        wybierz_jezyk(&term, &mut g);
        zapisz_ustawienia(&g);
    }
    g.komunikat = tr(g.jezyk, K::Welcome).to_string();

    let mut wybrane = 0usize;
    let mut upadek = false;

    loop {
        let wybor = menu_glowne(&term, &g, &mut wybrane);
        g.komunikat.clear();
        let j = g.jezyk;

        match wybor {
            0 => {
                let wolne = (0..POLA).any(|i| sprawdz_pole(&g, i, Tryb::Budowa).is_none());
                if !wolne {
                    g.komunikat = tr(j, K::NoFreePlot).to_string();
                } else if let Some(typ) = wybierz_budynek(&term, &g) {
                    match wybierz_pole(&term, &g, Tryb::Budowa, Some(typ)) {
                        Some(m) => {
                            g.pieniadze -= TYPY[typ].cena;
                            g.mapa[m] = Pole {
                                stan: Stan::Budowa,
                                typ,
                                hp: 100,
                                dni: TYPY[typ].czas_budowy,
                                moc: 0,
                            };
                            g.komunikat = fill(tr(j, K::ConstructionStarted), &[&TYPY[typ].nazwa(j)]);
                        }
                        None => g.komunikat = tr(j, K::Cancelled).to_string(),
                    }
                }
            }
            1 => {
                let wolne = (0..POLA).filter(|&i| !g.dzialki[i]).count();
                if wolne == 0 {
                    g.komunikat = tr(j, K::AllLandOwned).to_string();
                } else if let Some(m) = wybierz_dzialke(&term, &g) {
                    let cena = g.cena_dzialki();
                    if g.pieniadze < cena {
                        g.komunikat = tr(j, K::NoMoneyLand).to_string();
                    } else {
                        g.pieniadze -= cena;
                        g.dzialki[m] = true;
                        g.komunikat = fill(tr(j, K::LandBought), &[&cena]);
                    }
                } else {
                    g.komunikat = tr(j, K::Cancelled).to_string();
                }
            }
            2 => {
                if g.kredyt > 0 {
                    g.komunikat = tr(j, K::LoanActive).to_string();
                } else {
                    g.pieniadze += LOAN_AMOUNT;
                    g.kredyt = LOAN_AMOUNT;
                    g.do_splaty = LOAN_AMOUNT * (100 + LOAN_INTEREST) / 100;
                    g.komunikat = fill(tr(j, K::LoanTaken), &[&LOAN_AMOUNT, &g.do_splaty]);
                }
            }
            3 => {
                if g.kredyt <= 0 {
                    g.komunikat = tr(j, K::NoLoan).to_string();
                } else if g.pieniadze < g.do_splaty {
                    g.komunikat = tr(j, K::NoMoneyLoan).to_string();
                } else {
                    g.pieniadze -= g.do_splaty;
                    g.kredyt = 0;
                    g.do_splaty = 0;
                    g.komunikat = tr(j, K::LoanRepaid).to_string();
                }
            }
            4 => {
                if g.pieniadze >= CENA_TARCZY {
                    g.pieniadze -= CENA_TARCZY;
                    g.tarcze += 1;
                    g.komunikat = tr(j, K::ShieldBought).to_string();
                } else {
                    g.komunikat = tr(j, K::NoMoneyShield).to_string();
                }
            }
            5 => {
                let uszkodzone = (0..POLA).any(|i| sprawdz_pole(&g, i, Tryb::Naprawa).is_none());
                if !uszkodzone {
                    g.komunikat = tr(j, K::NothingRepair).to_string();
                } else if g.pieniadze < CENA_NAPRAWY {
                    g.komunikat = tr(j, K::NoMoneyRepair).to_string();
                } else if let Some(m) = wybierz_pole(&term, &g, Tryb::Naprawa, None) {
                    g.mapa[m].hp = engine::oblicz_nowe_hp(g.mapa[m].hp, LECZENIE);
                    g.pieniadze -= CENA_NAPRAWY;
                    g.komunikat = fill(tr(j, K::Repaired), &[&TYPY[g.mapa[m].typ].nazwa(j)]);
                }
            }
            6 => {
                przejdz_dni(&mut g, 1);
                let dzien = g.dzien;
                let m = fill(tr(j, K::DayN), &[&dzien]);
                g.dodaj_komunikat(&m);
            }
            7 => {
                if potwierdz(&term, &g, tr(j, K::ConfirmDisaster)) {
                    katastrofa(&mut g);
                }
            }
            8 => {
                ekran_ustawien(&term, &mut g);
            }
            _ => {
                if potwierdz(&term, &g, tr(j, K::ConfirmExit)) {
                    break;
                }
            }
        }

        if g.ludzie <= 0 && g.dzien > 5 && !g.budowa_trwa() {
            upadek = true;
            break;
        }
    }

    // ekran koncowy
    let j = g.jezyk;
    print!("\x1b[2J\x1b[H{}{}", BOLD, CYAN);
    if upadek {
        println!("  {}", tr(j, K::CollapsedLong));
    }
    println!("  {}{}", tr(j, K::GameOver), RESET);
    print!("{}", podsumowanie(&g));
    print!("\n  {}", tr(j, K::PressAnyKey));
    let _ = io::stdout().flush();
    term.klawisz();

    drop(term);
    if upadek {
        println!("{}", tr(j, K::CollapsedShort));
    }
    println!("{}", tr(j, K::GameOver));
    print!("{}", podsumowanie(&g));
}
