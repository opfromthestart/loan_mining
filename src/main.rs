use std::{
    borrow::{Borrow, BorrowMut},
    ops::{AddAssign, Deref, DerefMut, Sub},
    str::FromStr,
};

use csv::ReaderBuilder;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
struct F64(f64);

impl Eq for F64 {}

impl Ord for F64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl FromStr for F64 {
    type Err = <f64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl Borrow<f64> for F64 {
    fn borrow(&self) -> &f64 {
        &self.0
    }
}
impl BorrowMut<f64> for F64 {
    fn borrow_mut(&mut self) -> &mut f64 {
        &mut self.0
    }
}
impl Deref for F64 {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        self.borrow()
    }
}
impl DerefMut for F64 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.borrow_mut()
    }
}

impl AddAssign<F64> for f64 {
    fn add_assign(&mut self, rhs: F64) {
        *self += rhs.0;
    }
}
impl AddAssign<&F64> for f64 {
    fn add_assign(&mut self, rhs: &F64) {
        *self += rhs.0;
    }
}

impl Sub<F64> for f64 {
    type Output = f64;

    fn sub(self, rhs: F64) -> Self::Output {
        self - rhs.0
    }
}
impl Sub<f64> for F64 {
    type Output = f64;

    fn sub(self, rhs: f64) -> Self::Output {
        self.0 - rhs
    }
}
impl Sub<&f64> for &F64 {
    type Output = f64;

    fn sub(self, rhs: &f64) -> Self::Output {
        self.0 - rhs
    }
}
impl Sub<&F64> for &F64 {
    type Output = f64;

    fn sub(self, rhs: &F64) -> Self::Output {
        self.0 - rhs.0
    }
}
impl From<f64> for F64 {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum Value {
    Number(F64),
    Category(String),
    None,
}

impl FromStr for Value {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 0 {
            Ok(Self::None)
        } else if let Ok(n) = s.parse() {
            Ok(Self::Number(n))
        } else {
            Ok(Self::Category(s.into()))
        }
    }
}

#[derive(Debug)]
enum ValueType {
    Number { mean: f64, sd: f64 },
    Category(usize),
}

#[derive(Debug)]
struct ValueTypes(pub Vec<ValueType>);

impl TryFrom<&Vec<Vec<Value>>> for ValueTypes {
    type Error = &'static str;

    fn try_from(value: &Vec<Vec<Value>>) -> Result<Self, Self::Error> {
        let mut transpose = vec![vec![]; value[0].len()];
        for v in value {
            for (j, e) in v.into_iter().enumerate() {
                transpose[j].push(e);
            }
        }
        transpose.iter_mut().for_each(|v| {
            v.sort_unstable();
            v.dedup();
        });
        println!("{transpose:?}");
        for i in transpose.iter() {
            if i.iter().any(|x| matches!(x, Value::Number(_)))
                && i.iter().any(|x| matches!(x, Value::Category(_)))
            {
                return Err("Both numbers and categories found");
            }
        }
        let mut types: Vec<ValueType> = transpose
            .into_iter()
            .map(|x| {
                x.into_iter().fold(ValueType::Category(0), |f, v| {
                    if matches!(v, Value::Number(_)) {
                        ValueType::Number { mean: 0.0, sd: 0.0 }
                    } else if let ValueType::Category(n) = f {
                        ValueType::Category(n + 1)
                    } else {
                        // Numeric missing
                        f
                    }
                })
            })
            .collect();
        for (i, t) in types
            .iter_mut()
            .enumerate()
            .filter(|(_, x)| matches!(x, ValueType::Number { mean: _, sd: _ }))
        {
            let mut mean = 0.0;
            let mut count = 0;
            for v in value {
                let Value::Number(n) = v[i] else {
                    continue;
                };
                mean += n;
                count += 1;
            }
            mean /= count as f64;
            let mut var = 0.0;
            for v in value {
                let Value::Number(n) = v[i] else {
                    continue;
                };
                var += (mean - n).powi(2);
            }
            *t = ValueType::Number {
                mean,
                sd: (var / (count as f64)).sqrt(),
            };
        }
        Ok(ValueTypes(types))
    }
}

#[derive(Debug)]
struct Corrs(Vec<f64>);

impl From<(&Vec<Value>, &Vec<Vec<Value>>)> for Corrs {
    fn from((target, pred): (&Vec<Value>, &Vec<Vec<Value>>)) -> Self {
        let types = ValueTypes::try_from(pred).unwrap();
        let (tmean, tsd, count) = {
            let mut tmean = 0.0;
            let mut count = 0;
            for i in target {
                let Value::Number(n) = i else {
                    panic!("Target was not number");
                };
                tmean += n;
                count += 1;
            }
            tmean /= count as f64;
            (tmean, (tmean * (1.0 - tmean)).sqrt(), count)
        };
        Corrs(
            (0..pred[0].len())
                .map(|i| -> f64 {
                    let predi: Vec<_> = pred.iter().map(|v| &v[i]).collect();
                    match types.0[i] {
                        ValueType::Number { mean, sd } => {
                            let mut xy = 0.0;
                            for (t, p) in target.iter().zip(predi.iter()) {
                                let n = match p {
                                    Value::Number(n) => *n,
                                    Value::Category(_) => {
                                        panic!("Category found in numeric variable")
                                    }
                                    Value::None => mean.into(),
                                };
                                let tn = match t {
                                    Value::Number(n) => *n,
                                    _ => {
                                        panic!("Target was not a number");
                                    }
                                };
                                xy += (tn - tmean) * (n - mean);
                            }
                            xy /= count as f64;
                            println!("{xy} {sd} {tsd}");
                            (xy / (sd * tsd)).powi(2)
                        }
                        ValueType::Category(n) => {
                            let mut freqs: Vec<((Option<&String>, F64), f64)> = vec![];
                            for (tl, cl) in target.iter().zip(predi.iter()) {
                                let Value::Number(tl) = tl else {
                                    panic!("Target wasnt a number");
                                };
                                let pl = match cl {
                                    Value::Category(n) => Some(n),
                                    Value::None => None,
                                    Value::Number(_) => {
                                        unreachable!("Number found in categorical variable")
                                    }
                                };
                                match freqs
                                    .iter_mut()
                                    .find(|((plc, tlc), _)| tl == tlc && plc == &pl)
                                {
                                    Some((_, n)) => {
                                        *n += 1.0;
                                    }
                                    None => {
                                        freqs.push(((pl, *tl), 1.0));
                                    }
                                }
                            }
                            println!("{freqs:?}");
                            let mut freqp: Vec<(Option<&String>, f64)> = vec![];
                            let mut total = 0.0;
                            let mut yes = 0.0;
                            for ((pl, tl), c) in freqs.iter() {
                                match freqp.iter_mut().find(|(plc, _)| &plc == &pl) {
                                    Some((_, n)) => {
                                        *n += c;
                                    }
                                    None => {
                                        freqp.push((*pl, *c));
                                    }
                                };
                                total += c;
                                if tl.0 == 1.0 {
                                    yes += c;
                                }
                            }
                            assert_eq!(freqp.len(), n, "Number of categories is not correct.");
                            println!("{freqp:?}");
                            let mut expected: Vec<((Option<&String>, F64), f64)> = freqp
                                .iter()
                                .flat_map(|(l, c)| {
                                    // println!("{l:?} {c} {yes} {total}");
                                    [
                                        ((*l, F64(0.)), c * (total - yes) / total),
                                        ((*l, F64(1.)), c * yes / total),
                                    ]
                                })
                                .collect();
                            freqs.sort_by_key(|(k, _)| k.clone());
                            expected.sort_by_key(|(k, _)| k.clone());
                            println!("{freqs:?}\n{expected:?}");
                            let mut chisq = 0.0;
                            for ((p1, t1), c1) in expected.into_iter() {
                                let c2 = freqs
                                    .iter()
                                    .find(|x| &x.0 == &(p1, t1))
                                    .map(|(_, c)| *c)
                                    .unwrap_or(0.);
                                println!("{c1} {c2}");
                                chisq += (c1 - c2).powi(2) / c1;
                            }
                            // todo!("Confirm this formula");
                            (chisq / total / ((n - 1) as f64)).sqrt()
                        }
                    }
                    .sqrt()
                })
                .collect(),
        )
    }
}

fn record_dist(
    a: &Vec<Value>,
    b: &Vec<Value>,
    types: &ValueTypes,
    corrs: &Corrs,
    max: Option<f64>,
) -> f64 {
    let max = max.unwrap_or(f64::INFINITY);
    let mut dist = 0.0;
    for ((av, bv), (t, r2)) in a
        .iter()
        .zip(b.iter())
        .zip(types.0.iter().zip(corrs.0.iter()))
    {
        dist += match (av, bv) {
            (Value::Number(an), Value::Number(bn)) => {
                let ValueType::Number { mean: _, sd } = t else {
                    panic!("ValueType Number expected but {t:?} found");
                };
                (an - bn).abs() / sd
            }
            (Value::Number(n), Value::None) | (Value::None, Value::Number(n)) => {
                let ValueType::Number { mean, sd } = t else {
                    panic!("ValueType Number expected but {t:?} found");
                };
                (n - mean).abs() / sd
            }
            (Value::Category(_), Value::Category(_)) => todo!(),
            (Value::Category(_), Value::None) => todo!(),
            (Value::None, Value::Category(_)) => todo!(),
            (Value::None, Value::None) => todo!(),
            (Value::Number(_), Value::Category(_)) | (Value::Category(_), Value::Number(_)) => {
                unreachable!("Cannot have both numbers and categories")
            }
        } * r2;
        if dist > max {
            return dist;
        }
    }
    dist
}

fn main() {
    let loaded: Vec<Vec<Value>> = ReaderBuilder::new()
        .from_path("test.csv")
        .unwrap()
        .into_records()
        .into_iter()
        .map(|d| d.unwrap().into_iter().map(|x| x.parse().unwrap()).collect())
        .collect();
    println!("{loaded:?}");
    let targets: Vec<_> = loaded.iter().map(|v| v[1].clone()).collect();
    let preds: Vec<_> = loaded.iter().map(|v| v[2..].to_vec()).collect();
    let types = ValueTypes::try_from(&preds).unwrap();
    let corrs = Corrs::from((&targets, &preds));
    println!("{types:?} {corrs:?}");
}
