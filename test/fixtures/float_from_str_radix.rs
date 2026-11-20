// num-traits 0.2.19's `from_str_radix` of `f32` (MIT or Apache-2.0), its
// macro written out: a `match` whose arms meet at the merge from inside a
// block of the `match` before, past which tidy copies nothing (#2144).
#[derive(Debug)]
pub enum FloatErrorKind { Empty, Invalid }
#[derive(Debug)]
pub struct ParseFloatError { pub kind: FloatErrorKind }
fn str_to_ascii_lower_eq_str(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).all(|(a, b)| a.to_ascii_lowercase() == b)
}
pub struct F;
impl F {
            pub fn from_str_radix(src: &str, radix: u32)
                              -> Result<f32, ParseFloatError>
            {
                use FloatErrorKind::*;
                use ParseFloatError as PFE;

                // Special case radix 10 to use more accurate standard library implementation
                if radix == 10 {
                    return src.parse().map_err(|_| PFE {
                        kind: if src.is_empty() { Empty } else { Invalid },
                    });
                }

                // Special values
                if str_to_ascii_lower_eq_str(src, "inf")
                    || str_to_ascii_lower_eq_str(src, "infinity")
                {
                    return Ok(f32::INFINITY);
                } else if str_to_ascii_lower_eq_str(src, "-inf")
                    || str_to_ascii_lower_eq_str(src, "-infinity")
                {
                    return Ok(f32::NEG_INFINITY);
                } else if str_to_ascii_lower_eq_str(src, "nan") {
                    return Ok(f32::NAN);
                } else if str_to_ascii_lower_eq_str(src, "-nan") {
                    return Ok(-f32::NAN);
                }

                fn slice_shift_char(src: &str) -> Option<(char, &str)> {
                    let mut chars = src.chars();
                    Some((chars.next()?, chars.as_str()))
                }

                let (is_positive, src) =  match slice_shift_char(src) {
                    None             => return Err(PFE { kind: Empty }),
                    Some(('-', ""))  => return Err(PFE { kind: Empty }),
                    Some(('-', src)) => (false, src),
                    Some((_, _))     => (true,  src),
                };

                // The significand to accumulate
                let mut sig = if is_positive { 0.0 } else { -0.0 };
                // Necessary to detect overflow
                let mut prev_sig = sig;
                let mut cs = src.chars().enumerate();
                // Exponent prefix and exponent index offset
                let mut exp_info = None::<(char, usize)>;

                // Parse the integer part of the significand
                for (i, c) in cs.by_ref() {
                    match c.to_digit(radix) {
                        Some(digit) => {
                            // shift significand one digit left
                            sig *= radix as f32;

                            // add/subtract current digit depending on sign
                            if is_positive {
                                sig += (digit as isize) as f32;
                            } else {
                                sig -= (digit as isize) as f32;
                            }

                            // Detect overflow by comparing to last value, except
                            // if we've not seen any non-zero digits.
                            if prev_sig != 0.0 {
                                if is_positive && sig <= prev_sig
                                    { return Ok(f32::INFINITY); }
                                if !is_positive && sig >= prev_sig
                                    { return Ok(f32::NEG_INFINITY); }

                                // Detect overflow by reversing the shift-and-add process
                                if is_positive && (prev_sig != (sig - digit as f32) / radix as f32)
                                    { return Ok(f32::INFINITY); }
                                if !is_positive && (prev_sig != (sig + digit as f32) / radix as f32)
                                    { return Ok(f32::NEG_INFINITY); }
                            }
                            prev_sig = sig;
                        },
                        None => match c {
                            'e' | 'E' | 'p' | 'P' => {
                                exp_info = Some((c, i + 1));
                                break;  // start of exponent
                            },
                            '.' => {
                                break;  // start of fractional part
                            },
                            _ => {
                                return Err(PFE { kind: Invalid });
                            },
                        },
                    }
                }

                // If we are not yet at the exponent parse the fractional
                // part of the significand
                if exp_info.is_none() {
                    let mut power = 1.0;
                    for (i, c) in cs.by_ref() {
                        match c.to_digit(radix) {
                            Some(digit) => {
                                // Decrease power one order of magnitude
                                power /= radix as f32;
                                // add/subtract current digit depending on sign
                                sig = if is_positive {
                                    sig + (digit as f32) * power
                                } else {
                                    sig - (digit as f32) * power
                                };
                                // Detect overflow by comparing to last value
                                if is_positive && sig < prev_sig
                                    { return Ok(f32::INFINITY); }
                                if !is_positive && sig > prev_sig
                                    { return Ok(f32::NEG_INFINITY); }
                                prev_sig = sig;
                            },
                            None => match c {
                                'e' | 'E' | 'p' | 'P' => {
                                    exp_info = Some((c, i + 1));
                                    break; // start of exponent
                                },
                                _ => {
                                    return Err(PFE { kind: Invalid });
                                },
                            },
                        }
                    }
                }

                // Parse and calculate the exponent
                let exp = match exp_info {
                    Some((c, offset)) => {
                        let base = match c {
                            'E' | 'e' if radix == 10 => 10.0,
                            'P' | 'p' if radix == 16 => 2.0,
                            _ => return Err(PFE { kind: Invalid }),
                        };

                        // Parse the exponent as decimal integer
                        let src = &src[offset..];
                        let (is_positive, exp) = match slice_shift_char(src) {
                            Some(('-', src)) => (false, src.parse::<usize>()),
                            Some(('+', src)) => (true,  src.parse::<usize>()),
                            Some((_, _))     => (true,  src.parse::<usize>()),
                            None             => return Err(PFE { kind: Invalid }),
                        };


                        fn pow(base: f32, exp: usize) -> f32 {
                            f32::powi(base, exp as i32)
                        }
                        // otherwise uses the generic `pow` from the root

                        match (is_positive, exp) {
                            (true,  Ok(exp)) => pow(base, exp),
                            (false, Ok(exp)) => 1.0 / pow(base, exp),
                            (_, Err(_))      => return Err(PFE { kind: Invalid }),
                        }
                    },
                    None => 1.0, // no exponent
                };

                Ok(sig * exp)
            }
}
