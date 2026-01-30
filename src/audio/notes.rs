const SEMITONE: f64 = 1.059_463_094_36;
const A4_REF: f64 = 440.0;

// Compute relative to A4
const fn note_from_a4(offset: i32) -> f64 {
    let mut result = A4_REF;
    let mut i = 0;
    while i < offset.abs() {
        if offset > 0 {
            result *= SEMITONE;
        } else {
            result /= SEMITONE;
        }
        i += 1;
    }
    result
}

pub const REST: f64 = 0.0;

// Octave 0
pub const A0: f64 = note_from_a4(-48);
pub const AS0: f64 = note_from_a4(-47);
pub const B0: f64 = note_from_a4(-46);

// Octave 1
pub const C1: f64 = note_from_a4(-45);
pub const CS1: f64 = note_from_a4(-44);
pub const D1: f64 = note_from_a4(-43);
pub const DS1: f64 = note_from_a4(-42);
pub const E1: f64 = note_from_a4(-41);
pub const F1: f64 = note_from_a4(-40);
pub const FS1: f64 = note_from_a4(-39);
pub const G1: f64 = note_from_a4(-38);
pub const GS1: f64 = note_from_a4(-37);
pub const A1: f64 = note_from_a4(-36);
pub const AS1: f64 = note_from_a4(-35);
pub const B1: f64 = note_from_a4(-34);

// Octave 2
pub const C2: f64 = note_from_a4(-33);
pub const CS2: f64 = note_from_a4(-32);
pub const D2: f64 = note_from_a4(-31);
pub const DS2: f64 = note_from_a4(-30);
pub const E2: f64 = note_from_a4(-29);
pub const F2: f64 = note_from_a4(-28);
pub const FS2: f64 = note_from_a4(-27);
pub const G2: f64 = note_from_a4(-26);
pub const GS2: f64 = note_from_a4(-25);
pub const A2: f64 = note_from_a4(-24);
pub const AS2: f64 = note_from_a4(-23);
pub const B2: f64 = note_from_a4(-22);

// Octave 3
pub const C3: f64 = note_from_a4(-21);
pub const CS3: f64 = note_from_a4(-20);
pub const D3: f64 = note_from_a4(-19);
pub const DS3: f64 = note_from_a4(-18);
pub const E3: f64 = note_from_a4(-17);
pub const F3: f64 = note_from_a4(-16);
pub const FS3: f64 = note_from_a4(-15);
pub const G3: f64 = note_from_a4(-14);
pub const GS3: f64 = note_from_a4(-13);
pub const A3: f64 = note_from_a4(-12);
pub const AS3: f64 = note_from_a4(-11);
pub const B3: f64 = note_from_a4(-10);

// Octave 4
pub const C4: f64 = note_from_a4(-9);
pub const CS4: f64 = note_from_a4(-8);
pub const D4: f64 = note_from_a4(-7);
pub const DS4: f64 = note_from_a4(-6);
pub const E4: f64 = note_from_a4(-5);
pub const F4: f64 = note_from_a4(-4);
pub const FS4: f64 = note_from_a4(-3);
pub const G4: f64 = note_from_a4(-2);
pub const GS4: f64 = note_from_a4(-1);
pub const A4: f64 = note_from_a4(0);
pub const AS4: f64 = note_from_a4(1);
pub const B4: f64 = note_from_a4(2);

// Octave 5
pub const C5: f64 = note_from_a4(3);
pub const CS5: f64 = note_from_a4(4);
pub const D5: f64 = note_from_a4(5);
pub const DS5: f64 = note_from_a4(6);
pub const E5: f64 = note_from_a4(7);
pub const F5: f64 = note_from_a4(8);
pub const FS5: f64 = note_from_a4(9);
pub const G5: f64 = note_from_a4(10);
pub const GS5: f64 = note_from_a4(11);
pub const A5: f64 = note_from_a4(12);
pub const AS5: f64 = note_from_a4(13);
pub const B5: f64 = note_from_a4(14);

// Octave 6
pub const C6: f64 = note_from_a4(15);
