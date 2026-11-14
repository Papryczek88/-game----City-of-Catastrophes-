//! City of Catastrophes 8.0 - wersja Rust (tylko biblioteka standardowa).
//! Cale sterowanie: strzalki (lub W/A/S/D) + Enter + Esc.

mod engine;

use engine::Rng;
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
const BOLD: &str = "\x1b[1m";
const REV: &str = "\x1b[44;97m"; // ciemnoniebieskie tlo, bialy tekst
const EOL: &str = "\x1b[K\n";

const SZER: usize = 4;
const POLA: usize = SZER * SZER;
const CENA_TARCZY: i32 = 200;
const CENA_NAPRAWY: i32 = 50;
const LECZENIE: i32 = 25;
const SZER_LEWEJ: usize = 24;

struct Typ {
    nazwa: &'static str,
    znak: char,
    cena: i32,
    mieszkancy: i32,
    dochod: i32,
    atrakcja: i32,
    czas_budowy: i32,
    fabryka: bool,
}

static TYPY: [Typ; 7] = [
    Typ { nazwa: "House",            znak: 'H', cena: 50,  mieszkancy: 5,  dochod: 0,  atrakcja: 0,   czas_budowy: 1, fabryka: false },
    Typ { nazwa: "Apartment block",  znak: 'B', cena: 120, mieszkancy: 20, dochod: 1,  atrakcja: 0,   czas_budowy: 3, fabryka: false },
    Typ { nazwa: "Luxury apartment", znak: 'A', cena: 200, mieszkancy: 40, dochod: 2,  atrakcja: 10,  czas_budowy: 5, fabryka: false },
    Typ { nazwa: "Park",             znak: 'P', cena: 80,  mieszkancy: 0,  dochod: 1,  atrakcja: 15,  czas_budowy: 2, fabryka: false },
    Typ { nazwa: "Restaurant",       znak: 'R', cena: 150, mieszkancy: 5,  dochod: 5,  atrakcja: 20,  czas_budowy: 3, fabryka: false },
    Typ { nazwa: "Farm",             znak: 'F', cena: 60,  mieszkancy: 0,  dochod: 1,  atrakcja: 0,   czas_budowy: 2, fabryka: false },
    Typ { nazwa: "Factory",          znak: 'I', cena: 300, mieszkancy: 10, dochod: 10, atrakcja: -5,  czas_budowy: 7, fabryka: true },
];

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
}

const PUSTE: Pole = Pole { stan: Stan::Puste, typ: 0, hp: 0, dni: 0 };

struct Gra {
    mapa: [Pole; POLA],
    pieniadze: i32,
    ludzie: i32,
    atrakcja: i32,
    punkty: i32,
    dzien: i32,
    tarcze: i32,
    komunikat: String,
    rng: Rng,
}

impl Gra {
    fn nowa(seed: u64) -> Gra {
        Gra {
            mapa: [PUSTE; POLA],
            pieniadze: 500,
            ludzie: 0,
            atrakcja: 0,
            punkty: 0,
            dzien: 1,
            tarcze: 0,
            komunikat: "Welcome! Expand your city and watch out for disasters.".to_string(),
            rng: Rng::new(seed),
        }
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
}

#[cfg(windows)]
impl Terminal {
    fn new() -> Terminal {
        unsafe {
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
            Stan::Puste => (RESET, ' '),
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

/// Caly ekran: naglowek, mapa po lewej, kolumna "prawa" obok, linie "dol" pod spodem.
fn ekran(g: &Gra, kursor: Option<usize>, prawa: &[String], dol: &[String]) {
    let mut s = String::from("\x1b[H");
    ln(&mut s, &format!("{}{}  ===== CITY OF CATASTROPHES 8.0 ====={}", BOLD, CYAN, RESET));
    ln(
        &mut s,
        &format!(
            "  Day: {}   Money: {}   Residents: {}",
            kol(ZOLTY, &g.dzien.to_string()),
            kol(ZIEL, &g.pieniadze.to_string()),
            g.ludzie
        ),
    );
    ln(
        &mut s,
        &format!(
            "  Attractiveness: {}   Shields: {}   Points: {}",
            g.atrakcja, g.tarcze, g.punkty
        ),
    );
    ln(&mut s, "");

    let wiersze = prawa.len().max(6);
    for r in 0..wiersze {
        let lewa = if r == 0 {
            format!("{}  CITY MAP{}{}", BOLD, RESET, " ".repeat(SZER_LEWEJ - 10))
        } else if r <= SZER {
            format!("{}{}", wiersz_mapy(g, r - 1, kursor), " ".repeat(SZER_LEWEJ - 18))
        } else {
            " ".repeat(SZER_LEWEJ)
        };
        let p = prawa.get(r).map(String::as_str).unwrap_or("");
        ln(&mut s, &format!("{}{}", lewa, p));
    }

    ln(&mut s, "  H House  B Block  A Apartment  P Park");
    ln(&mut s, "  R Restaurant  F Farm  I Industry  # Under construction");

    let budowy: Vec<String> = g
        .mapa
        .iter()
        .filter(|p| p.stan == Stan::Budowa)
        .map(|p| format!("{} ({}d)", TYPY[p.typ].nazwa, p.dni))
        .collect();
    let budowy = if budowy.is_empty() { "none".to_string() } else { budowy.join(", ") };
    ln(&mut s, &format!("  Under construction: {}", budowy));

    for linia in dol {
        ln(&mut s, linia);
    }
    s.push_str("\x1b[J");

    let mut wyj = io::stdout().lock();
    let _ = wyj.write_all(s.as_bytes());
    let _ = wyj.flush();
}

// ---------------------------------------------------------------- logika

fn przejdz_dni(g: &mut Gra, liczba_dni: i32) {
    for _ in 0..liczba_dni {
        g.dzien += 1;
        for i in 0..POLA {
            match g.mapa[i].stan {
                Stan::Gotowe => {
                    let p = g.mapa[i];
                    g.pieniadze += engine::oblicz_dochod_hp(
                        engine::oblicz_dochod(TYPY[p.typ].dochod),
                        p.hp,
                    );
                }
                Stan::Budowa => {
                    g.mapa[i].dni -= 1;
                    if g.mapa[i].dni <= 0 {
                        g.mapa[i].stan = Stan::Gotowe;
                        let t = &TYPY[g.mapa[i].typ];
                        g.ludzie += t.mieszkancy;
                        g.atrakcja += t.atrakcja;
                        g.punkty += 10;
                        g.dodaj_komunikat(&format!("Completed: {}.", t.nazwa));
                    }
                }
                Stan::Puste => {}
            }
        }
    }
}

fn katastrofa(g: &mut Gra) {
    const NAZWY: [&str; 6] = ["Flood", "Earthquake", "Meteor", "Hurricane", "Fire", "Factory accident"];
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
        g.dodaj_komunikat("The shield halved the damage.");
    }
    g.dodaj_komunikat(&format!("{}! Damage: -{} HP.", NAZWY[los], obrazenia));

    for i in 0..POLA {
        if g.mapa[i].stan != Stan::Gotowe {
            continue;
        }
        let w = engine::wykonaj_katastrofe(g.mapa[i].hp, obrazenia);
        g.mapa[i].hp = w.hp;
        if w.zniszczony {
            let t = &TYPY[g.mapa[i].typ];
            g.dodaj_komunikat(&format!("Destroyed: {}.", t.nazwa));
            g.ludzie -= t.mieszkancy;
            g.atrakcja -= t.atrakcja;
            g.mapa[i] = PUSTE;
        }
    }
    g.ludzie = g.ludzie.max(0);
    g.atrakcja = g.atrakcja.max(0);

    przejdz_dni(g, DNI[los]);
    g.dodaj_komunikat(&format!("The disaster cost {} days.", DNI[los]));
    g.punkty += 30;
}

// ---------------------------------------------------------------- menu

fn menu_glowne(t: &Terminal, g: &Gra, wybrane: &mut usize) -> usize {
    const POZYCJE: [&str; 6] = [
        "Buy a building",
        "Buy disaster shield (200)",
        "Repair a building (50)",
        "Next day",
        "Cause a disaster",
        "Exit",
    ];
    let n = POZYCJE.len();

    loop {
        let mut prawa = vec![kol(BOLD, "MENU")];
        for (i, nazwa) in POZYCJE.iter().enumerate() {
            if i == *wybrane {
                prawa.push(format!("{}> {:<28}{}", REV, nazwa, RESET));
            } else {
                prawa.push(format!("  {:<28}", nazwa));
            }
        }
        let mut dol = vec![String::new()];
        if !g.komunikat.is_empty() {
            dol.push(format!("  {}", kol(ZOLTY, &g.komunikat)));
        }
        dol.push(String::new());
        dol.push(hint("Up/Down: choose | Enter: confirm | Esc: exit"));
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
    let mut tak = false;
    loop {
        let opcje = if tak {
            format!("  No    {} > Yes {}", REV, RESET)
        } else {
            format!("  {} > No {}    Yes", REV, RESET)
        };
        let prawa = vec![kol(BOLD, "CONFIRM")];
        let dol = vec![
            String::new(),
            format!("  {}", kol(ZOLTY, pytanie)),
            opcje,
            String::new(),
            hint("Left/Right: choose | Enter: confirm | Esc: cancel"),
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
    let mut wybrane = 0usize;
    let mut blad = String::new();
    loop {
        let mut prawa = vec![kol(BOLD, "BUILDING SHOP")];
        for (i, ty) in TYPY.iter().enumerate() {
            let tekst = format!(
                "{}{:<17}{:<5}{}d",
                if i == wybrane { "> " } else { "  " },
                ty.nazwa,
                ty.cena,
                ty.czas_budowy
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
        let dol = vec![
            String::new(),
            format!(
                "  Residents: {}   Income: {}/day   Attractiveness: {}{}",
                ty.mieszkancy, ty.dochod, plus, ty.atrakcja
            ),
            format!("  {}", kol(CZERW, &blad)),
            hint("Up/Down: choose | Enter: buy | Esc: back"),
        ];
        ekran(g, None, &prawa, &dol);

        blad.clear();
        match t.klawisz() {
            Klawisz::Gora => wybrane = (wybrane + TYPY.len() - 1) % TYPY.len(),
            Klawisz::Dol => wybrane = (wybrane + 1) % TYPY.len(),
            Klawisz::Esc => return None,
            Klawisz::Enter => {
                if TYPY[wybrane].cena > g.pieniadze {
                    blad = "You do not have enough money.".to_string();
                } else {
                    return Some(wybrane);
                }
            }
            _ => {}
        }
    }
}

/// None = pole poprawne, Some(powod) = odmowa.
fn sprawdz_pole(g: &Gra, i: usize, tryb: Tryb) -> Option<&'static str> {
    let p = &g.mapa[i];
    if tryb == Tryb::Budowa {
        return if p.stan == Stan::Puste { None } else { Some("This tile is already occupied.") };
    }
    if p.stan != Stan::Gotowe {
        return Some("There is no finished building here.");
    }
    if p.hp >= 100 {
        return Some("This building is undamaged.");
    }
    None
}

fn wybierz_pole(t: &Terminal, g: &Gra, tryb: Tryb, typ: Option<usize>) -> Option<usize> {
    let mut kursor = (0..POLA)
        .find(|&i| sprawdz_pole(g, i, tryb).is_none())
        .unwrap_or(0);
    let mut blad = String::new();

    loop {
        let p = &g.mapa[kursor];
        let tytul = if tryb == Tryb::Budowa { "CHOOSE A CONSTRUCTION SITE" } else { "CHOOSE A BUILDING TO REPAIR" };
        let mut prawa = vec![kol(BOLD, tytul)];
        if let (Tryb::Budowa, Some(ty)) = (tryb, typ) {
            prawa.push(format!("Placing: {}", TYPY[ty].nazwa));
        }
        match p.stan {
            Stan::Puste => prawa.push("Empty tile".to_string()),
            Stan::Budowa => {
                prawa.push(format!("{} (under construction)", TYPY[p.typ].nazwa));
                prawa.push(format!("Days remaining: {}", p.dni));
            }
            Stan::Gotowe => {
                let ty = &TYPY[p.typ];
                prawa.push(ty.nazwa.to_string());
                prawa.push(pasek_hp(p.hp));
                prawa.push(format!(
                    "Income: {}/day (max {})",
                    engine::oblicz_dochod_hp(ty.dochod, p.hp),
                    ty.dochod
                ));
                prawa.push(format!("Residents: {}", ty.mieszkancy));
            }
        }
        let dol = vec![
            String::new(),
            format!("  {}", kol(CZERW, &blad)),
            hint("Arrows: move | Enter: confirm | Esc: cancel"),
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

fn podsumowanie(g: &Gra) -> String {
    format!(
        "  Days: {}\n  Points: {}\n  Residents: {}\n  Money: {}\n  Attractiveness: {}\n",
        g.dzien, g.punkty, g.ludzie, g.pieniadze, g.atrakcja
    )
}

fn main() {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(12345);
    let mut g = Gra::nowa(seed);
    let term = Terminal::new();
    let mut wybrane = 0usize;
    let mut upadek = false;

    loop {
        let wybor = menu_glowne(&term, &g, &mut wybrane);
        g.komunikat.clear();

        match wybor {
            0 => {
                let zajete = g.mapa.iter().filter(|p| p.stan != Stan::Puste).count();
                if zajete >= POLA {
                    g.komunikat = "The city is full.".to_string();
                } else if let Some(typ) = wybierz_budynek(&term, &g) {
                    match wybierz_pole(&term, &g, Tryb::Budowa, Some(typ)) {
                        Some(m) => {
                            g.pieniadze -= TYPY[typ].cena;
                            g.mapa[m] = Pole {
                                stan: Stan::Budowa,
                                typ,
                                hp: 100,
                                dni: TYPY[typ].czas_budowy,
                            };
                            g.komunikat = format!("Construction started: {}.", TYPY[typ].nazwa);
                        }
                        None => g.komunikat = "Cancelled.".to_string(),
                    }
                }
            }
            1 => {
                if g.pieniadze >= CENA_TARCZY {
                    g.pieniadze -= CENA_TARCZY;
                    g.tarcze += 1;
                    g.komunikat = "Disaster shield purchased.".to_string();
                } else {
                    g.komunikat = "You do not have enough money for a shield.".to_string();
                }
            }
            2 => {
                let uszkodzone = (0..POLA).any(|i| sprawdz_pole(&g, i, Tryb::Naprawa).is_none());
                if !uszkodzone {
                    g.komunikat = "Nothing to repair.".to_string();
                } else if g.pieniadze < CENA_NAPRAWY {
                    g.komunikat = "You do not have enough money for repairs.".to_string();
                } else if let Some(m) = wybierz_pole(&term, &g, Tryb::Naprawa, None) {
                    g.mapa[m].hp = engine::oblicz_nowe_hp(g.mapa[m].hp, LECZENIE);
                    g.pieniadze -= CENA_NAPRAWY;
                    g.komunikat = format!("Repaired: {}.", TYPY[g.mapa[m].typ].nazwa);
                }
            }
            3 => {
                przejdz_dni(&mut g, 1);
                let dzien = g.dzien;
                g.dodaj_komunikat(&format!("Day {}.", dzien));
            }
            4 => {
                if potwierdz(&term, &g, "Cause a disaster? Buildings will be damaged (+30 points).") {
                    katastrofa(&mut g);
                }
            }
            _ => {
                if potwierdz(&term, &g, "Exit the game?") {
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
    print!("\x1b[2J\x1b[H{}{}", BOLD, CYAN);
    if upadek {
        println!("  The city has collapsed - no residents remain.");
    }
    println!("  GAME OVER{}", RESET);
    print!("{}", podsumowanie(&g));
    print!("\n  Press any key...");
    let _ = io::stdout().flush();
    term.klawisz();

    drop(term);
    if upadek {
        println!("The city has collapsed.");
    }
    println!("GAME OVER");
    print!("{}", podsumowanie(&g));
}
