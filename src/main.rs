use std::collections::{BTreeSet, HashSet};
use std::io::Write;

use std::{
    borrow::{Borrow, BorrowMut},
    fs::File,
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

impl TryFrom<&[Vec<Value>]> for ValueTypes {
    type Error = &'static str;

    fn try_from(value: &[Vec<Value>]) -> Result<Self, Self::Error> {
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
        // println!("{transpose:?}");
        write!(
            std::fs::File::create("all_vals.txt").unwrap(),
            "{:?}",
            transpose
                .iter()
                .enumerate()
                .filter(|(_, x)| x.iter().any(|v| matches!(v, Value::Category(_))))
                .collect::<Vec<_>>()
        )
        .unwrap();
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

impl From<(&[Value], &[Vec<Value>])> for Corrs {
    fn from((target, pred): (&[Value], &[Vec<Value>])) -> Self {
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
                            // println!("{xy} {sd} {tsd}");
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
                            // println!("{freqs:?}");
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
                            // println!("{freqp:?}");
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
                            // println!("{freqs:?}\n{expected:?}");
                            let mut chisq = 0.0;
                            for ((p1, t1), c1) in expected.into_iter() {
                                let c2 = freqs
                                    .iter()
                                    .find(|x| &x.0 == &(p1, t1))
                                    .map(|(_, c)| *c)
                                    .unwrap_or(0.);
                                // println!("{c1} {c2}");
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
    // Allows checking high variance dimensions first to short circuit fast.
    order: Option<&[usize]>,
    max_check: Option<usize>,
) -> f64 {
    let max = max.unwrap_or(f64::INFINITY);
    let mut dist = 0.0;
    let poss_order = if order.is_none() {
        Some((0..a.len()).collect::<Vec<_>>())
    } else {
        None
    };
    let order = order
        .or(poss_order.as_deref())
        .expect("One of these must be some");
    let max_check = max_check.unwrap_or(order.len());
    for i in 0..max_check {
        let av = &a[order[i]];
        let bv = &b[order[i]];
        let t = &types.0[order[i]];
        let r = &corrs.0[order[i]];

        dist += match (av, bv) {
            (Value::Number(an), Value::Number(bn)) => {
                let ValueType::Number { mean: _, sd } = t else {
                    panic!("ValueType Number expected but {t:?} found");
                };
                (an - bn).abs() / sd
            }
            // (Value::Number(n), Value::None) | (Value::None, Value::Number(n)) => {
            //     let ValueType::Number { mean, sd } = t else {
            //         panic!("ValueType Number expected but {t:?} found");
            //     };
            //     (n - mean).abs() / sd
            // }
            (Value::Category(n), Value::Category(m)) => {
                if n == m {
                    0.
                } else {
                    1.
                }
            }
            // (Value::Category(_), Value::None) | (Value::None, Value::Category(_)) => 1.,
            // (Value::None, Value::None) => 0.,
            (_, Value::None) | (Value::None, _) => 0.,
            (Value::Number(_), Value::Category(_)) | (Value::Category(_), Value::Number(_)) => {
                unreachable!("Cannot have both numbers and categories")
            }
        } * r;
        if dist > max {
            return dist;
        }
    }
    dist
}

struct PrioN<T>(pub Vec<(T, f64)>, pub usize);

impl<T> PrioN<T> {
    fn insert(&mut self, i: f64, v: T) {
        let ip = self
            .0
            .iter()
            .position(|(_, t)| &i < t)
            .unwrap_or(self.0.len());
        self.0.insert(ip, (v, i));
        if self.0.len() > self.1 {
            self.0.remove(self.1);
        }
    }

    fn new(size: usize) -> Self {
        Self(vec![], size)
    }

    fn worst(&self) -> Option<f64> {
        self.0.last().map(|(_, f)| *f)
    }
}

fn knn(
    a: &Vec<Value>,
    pop: &[Vec<Value>],
    targets: &[Value],
    types: &ValueTypes,
    corrs: &Corrs,
    // Allows checking high variance dimensions first to short circuit fast.
    order: Option<&[usize]>,
    n: usize,
    max_check: Option<usize>,
) -> f64 {
    let mut prio: PrioN<usize> = PrioN::new(n);
    for (i, b) in pop.iter().enumerate() {
        prio.insert(
            record_dist(a, b, types, corrs, prio.worst(), order, max_check),
            i,
        );
        // println!("{}", prio.0.len());
    }
    prio.0
        .into_iter()
        .map(|(v, _)| &targets[v])
        .map(|t| {
            let Value::Number(tn) = t else {
                panic!("Target was not number in knn");
            };
            tn.0 / (n as f64)
        })
        .sum()
}

fn prompt_record() -> Vec<Value> {
    let mut rec = vec![Value::None; 120];
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    print!("Gender: M/F >");
    stdout.flush().unwrap();
    let mut s = String::new();
    stdin.read_line(&mut s).unwrap();
    rec[1] = Value::Category(s);

    print!("Contract type: Cash loans/Revolving loans >");
    stdout.flush().unwrap();
    let mut s = String::new();
    stdin.read_line(&mut s).unwrap();
    rec[0] = Value::Category(s);

    print!("Emergency state: Yes/No >");
    stdout.flush().unwrap();
    let mut s = String::new();
    stdin.read_line(&mut s).unwrap();
    rec[88] = Value::Category(s);

    rec
}

fn main() {
    let mut file = ReaderBuilder::new()
        .from_path(std::env::args().nth(1).expect("Must include document path"))
        .unwrap();
    let headers: Vec<_> = file.headers().unwrap().iter().map(String::from).collect();
    let loaded: Vec<Vec<Value>> = file
        .into_records()
        .into_iter()
        .map(|d| d.unwrap().into_iter().map(|x| x.parse().unwrap()).collect())
        .collect();
    // println!("{loaded:?}");
    let targets: Vec<_> = loaded.iter().map(|v| v[1].clone()).collect();
    println!("Loaded targets");
    let preds: Vec<_> = loaded.iter().map(|v| v[2..].to_vec()).collect();
    println!("Loaded variables");
    let (targets, test) = targets.split_at(targets.len() * 29 / 30);
    let (preds, preds_test) = preds.split_at(preds.len() * 29 / 30);
    // println!("{preds:?}");
    let types = ValueTypes::try_from(preds as &[_]).unwrap();
    println!("Identified types");
    let corrs = Corrs::from((targets as &[_], preds as &[_]));
    // println!("{types:?}\n{corrs:?}");
    write!(
        File::create("app_data_corrs.txt").unwrap(),
        "{types:?}\n{corrs:?}\n"
    )
    .unwrap();
    let mut cor_ind: Vec<_> = corrs
        .0
        .iter()
        .enumerate()
        .map(|(i, c)| (i, c, &headers[i + 2]))
        .collect();
    cor_ind.sort_by_key(|(_, v, _)| F64(-*v));
    write!(
        File::create("app_data_corrs_rank.txt").unwrap(),
        "{cor_ind:?}\n"
    )
    .unwrap();
    // cor_ind.iter().for_each(|(x, _, _)| print!("{x}, "));
    let order: Vec<_> = cor_ind.into_iter().map(|(i, _, _)| i).collect();
    // {
    //     println!("Computing error");
    //     let mut err = 0.0;
    //     let mut i = 0;
    //     let mut prop = 0.;
    //     let mut test_freq = BTreeSet::new();
    //     for (a, t) in preds_test.iter().zip(test.iter()) {
    //         i += 1;
    //         // println!("{i}");
    //         if i % 10 == 0 {
    //             print!("\r{i}");
    //             std::io::stdout().flush().unwrap();
    //         }
    //         let Value::Number(t) = t else {
    //             panic!("Category in target variable");
    //         };
    //         let k = knn(
    //             a,
    //             preds as &[_],
    //             targets as &[_],
    //             &types,
    //             &corrs,
    //             Some(&order),
    //             16,
    //             None,
    //         );
    //         test_freq.insert(F64(k));
    //         // println!("{k} {t:?}");
    //         err += (k - t.0).powi(2);
    //         prop += t;
    //     }
    //     prop /= preds.len() as f64;
    //     err = (err / (preds.len() as f64)).sqrt();
    //     println!("Err: {err}, worst={}", (prop * (1.0 - prop)).sqrt());
    //     println!("{test_freq:?}");
    // }
    loop {
        println!("Input borrower data");
        let ir = prompt_record();
        let k = knn(&ir, preds, targets, &types, &corrs, Some(&order), 30, None);
        println!("Prediction for borrower default is {k}");
    }
}
