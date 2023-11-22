use std::str::FromStr;

use csv::ReaderBuilder;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
enum Value {
    Number(f64),
    Category(String),
    None,
}

impl Eq for Value {}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
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
                sd: var.sqrt(),
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
                .map(|i| {
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
                                    Value::None => mean,
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
                            xy / (sd * tsd)
                        }
                        ValueType::Category(_) => todo!(),
                    }
                })
                .collect(),
        )
    }
}

fn record_dist(a: &Vec<Value>, b: &Vec<Value>, types: &ValueTypes, max: Option<f64>) -> f64 {
    let max = max.unwrap_or(f64::INFINITY);
    let mut dist = 0.0;
    for ((av, bv), t) in a.iter().zip(b.iter()).zip(types.0.iter()) {
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
        };
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
    println!("{:?}", ValueTypes::try_from(&preds));
}
