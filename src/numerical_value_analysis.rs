use std::collections::HashMap;
use parse::*;
use numerical_value::*;
use bounded_value::*;
extern crate serde;
extern crate serde_json;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    location: String,
    always_true: bool,
}

pub fn analyze(graph: &Graph) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    numerical_value_analysis(graph, graph.first(), &mut HashMap::new(), &mut HashMap::new(), &mut diagnostics);
    diagnostics
}

#[derive(PartialEq, Eq, Debug)]
struct VariableValueSlice<T> {
    name: String,
    pass: Range<T>,
    fail: Range<T>
}

fn numerical_value_analysis(graph: &Graph, location: &str,
                            variables: &mut HashMap<String, NumericalValue<BoundedValue<i64>>>,
                            history: &mut HashMap<String, HashMap<String, NumericalValue<BoundedValue<i64>>>>,
                            diagnostics: &mut Vec<Diagnostic>) {
    let node = graph.value_of(location).unwrap();
    match history.get(location) {
        Some(location_history) => {
            let mut any_changed = false;
            for (key, value) in location_history {
                if variables.contains_key(key) {
                    let (new_var, eq) = {
                        let var = &variables[key];
                        let new_var = var.union(value);
                        let eq = *var != new_var;
                        (new_var, eq)
                    };
                    if eq {
                        variables.insert(key.clone(), new_var);
                        any_changed = true;
                    }
                } else {
                    variables.insert(key.clone(), value.clone());
                    any_changed = true;
                }
            }
            if history[location].is_empty() && !variables.is_empty() {
                any_changed = true;
            }
            if !any_changed {
                return;
            }
        },
        None => {}
    }
    history.insert(location.to_string(), variables.clone());

    let mut slices = Vec::new();
    match node {
        NodeValue::VariableDeclaration { declarations } => {
            for declaration in declarations {
                let parsed = parse_value_expression(&declaration.initializer, &variables);
                variables.insert(declaration.identifier.clone(), parsed);
            }
        },
        NodeValue::VariableAssignment { left, right } => {
            let parsed = parse_value_expression(&right, &variables);
            variables.insert(left.clone(), parsed);
        },
        NodeValue::Comparison { left, op, right } => {
            handle_comparison(location, left, op, right, variables, &mut slices, diagnostics);
        },
        NodeValue::Other => {},
    }

    println!("{} -> {:?} ({})", location, variables, node);

    for succ in graph.successors_of(location).unwrap() {
        let mut vars = variables.clone();
        for slice in slices.iter() {
            let new_var = vars[&slice.name].intersect_range(
                if succ.value == 1 { &slice.pass }
                else if succ.value == 0 { &slice.fail }
                else { unreachable!() });
            vars.insert(slice.name.clone(), new_var);
        }
        numerical_value_analysis(graph, &succ.key, &mut vars, history, diagnostics);
    }
}

fn parse_value_expression(node: &Expression, variables: &HashMap<String, NumericalValue<BoundedValue<i64>>>)
                          -> NumericalValue<BoundedValue<i64>> {
    use Expression::*;
    match node {
        Binary { left, op, right } => {
            let l = parse_value_expression(left, variables).range().unwrap();
            let r = parse_value_expression(right, variables).range().unwrap();
            NumericalValue::from(match op.as_str() {
                "+" => l + r,
                "-" => l - r,
                "*" => l * r,
                "/" => l / r,
                "%" => l % r,
                _ => unreachable!(),
            })
        },
        Number(num) => NumericalValue::from(BoundedValue::Raw(*num)),
        Identifier(var) => variables[var].clone(),
        Other => NumericalValue::new_value(BoundedValue::Min, Inclusivity::Inclusive,
                                           BoundedValue::Max, Inclusivity::Inclusive),
    }
}

#[derive(Clone, Copy, Debug)]
enum ComparisonOperator {
    Less, LessEqual, Greater, GreaterEqual, Equals, NotEquals,
}
impl ComparisonOperator {
    fn flip(self) -> Self {
        use self::ComparisonOperator::*;
        match self {
            Less => Greater,
            LessEqual => GreaterEqual,
            Greater => Less,
            GreaterEqual => LessEqual,
            Equals | NotEquals => self,
        }
    }

    fn not(self) -> Self {
        use self::ComparisonOperator::*;
        match self {
            Less => GreaterEqual,
            LessEqual => Greater,
            Greater => LessEqual,
            GreaterEqual => Less,
            Equals => NotEquals,
            NotEquals => Equals,
        }
    }
}

fn descend(node: &Expression, range: Range<BoundedValue<i64>>, cmp_op: ComparisonOperator,
           variables: &HashMap<String, NumericalValue<BoundedValue<i64>>>,
           slices: &mut Vec<VariableValueSlice<BoundedValue<i64>>>) {
    use Expression::*;
    use Inclusivity::*;
    match node {
        Expression::Identifier(name) => {
            use self::ComparisonOperator::*;
            let e = parse_value_expression(node, variables);
            let pr;
            let fr;
            match cmp_op {
                Less => {
                    let (max_v, max_i) =
                        if range.max.inclusivity == Inclusive {
                            (range.max.value, Exclusive)
                        } else {
                            (range.max.value + (-1).into(), Exclusive)
                        };
                    pr = Range::new(BoundedValue::Min, Inclusive, max_v, max_i);
                    fr = Range::new(range.min.value, range.min.inclusivity,
                                    BoundedValue::Max, Inclusive);
                },
                LessEqual => {
                    let (min_v, min_i) =
                        if range.min.inclusivity == Inclusive {
                            (range.min.value, Exclusive)
                        } else {
                            (range.min.value + 1.into(), Exclusive)
                        };
                    pr = Range::new(BoundedValue::Min, Inclusive,
                                    range.max.value, range.max.inclusivity);
                    fr = Range::new(min_v, min_i,
                                    BoundedValue::Max, Inclusive);
                },
                Greater => {
                    let (min_v, min_i) =
                        if range.min.inclusivity == Inclusive {
                            (range.min.value, Exclusive)
                        } else {
                            (range.min.value + 1.into(), Exclusive)
                        };
                    pr = Range::new(min_v, min_i,
                                    BoundedValue::Max, Inclusive);
                    fr = Range::new(BoundedValue::Min, Inclusive,
                                    range.max.value, range.max.inclusivity);
                },
                GreaterEqual => {
                    let (max_v, max_i) =
                        if range.max.inclusivity == Inclusive {
                            (range.max.value, Exclusive)
                        } else {
                            (range.max.value + (-1).into(), Exclusive)
                        };
                    pr = Range::new(range.min.value, range.min.inclusivity,
                                    BoundedValue::Max, Inclusive);
                    fr = Range::new(BoundedValue::Min, Inclusive,
                                    max_v, max_i);
                },
                Equals => {
                    pr = range;
                    fr = Range::universe();
                },
                NotEquals => {
                    pr = Range::universe();
                    fr = range;
                }
            }
            match (e.intersect_range(&pr).range(), e.intersect_range(&fr).range()) {
                (Some(pass), Some(fail)) =>
                    slices.push(VariableValueSlice {
                        name: name.clone(), pass, fail,
                    }),
                _ => {},
            }
        },
        Expression::Binary { left, op, right } => {
            let l = parse_value_expression(left, variables).range().unwrap();
            let r = parse_value_expression(right, variables).range().unwrap();
            match op.as_str() {
                "+" => {
                    // l + r < range
                    // l < range - r
                    // r < range - l
                    descend(left, range - r, cmp_op, variables, slices);
                    descend(right, range - l, cmp_op, variables, slices);
                },
                "-" => {
                    // l - r < range
                    // l < range + r
                    // r > l - range
                    descend(left, range + r, cmp_op, variables, slices);
                    descend(right, l - range, cmp_op.flip(), variables, slices);
                },
                "*" => {
                    // l * r < range
                    // l < range / r
                    // r < range / l
                    descend(left, range / r, cmp_op, variables, slices);
                    descend(right, range / l, cmp_op, variables, slices);
                },
                "/" => {
                    // l / r < range
                    // l < range * r
                    // r > l / range
                    descend(left, range * r, cmp_op, variables, slices);
                    descend(right, l / range, cmp_op.flip(), variables, slices);
                }
                "%" => {
                    // l % r < range
                    // r < range
                    descend(right, range, cmp_op, variables, slices);
                }
                _ => unreachable!(),
            }
        },
        Number(_) => {},
        Other => {},
    }
}

fn handle_comparison(location: &str, left: &Expression, cmp_op: &str, right: &Expression,
                     variables: &HashMap<String, NumericalValue<BoundedValue<i64>>>,
                     slices: &mut Vec<VariableValueSlice<BoundedValue<i64>>>,
                     diagnostics: &mut Vec<Diagnostic>) {
    use self::ComparisonOperator::*;
    let cmp_op = match cmp_op {
        "<" => Less,
        "<=" => LessEqual,
        ">" => Greater,
        ">=" => GreaterEqual,
        "==" => Equals,
        "!=" => NotEquals,
        _ => unimplemented!(),
    };
    let l = parse_value_expression(left, variables);
    let r = parse_value_expression(right, variables);
    let l = l.range().unwrap();
    let r = r.range().unwrap();
    descend(left, r, cmp_op, variables, slices);
    descend(right, l, cmp_op.flip(), variables, slices);
    let always_true = match cmp_op {
        Less => l.max < r.min,
        LessEqual => l.max <= r.min,
        Greater => l.min > r.max,
        GreaterEqual => l.min >= r.max,
        Equals => l == r,
        NotEquals => l.max < r.min || l.min > r.max,
    };
    let always_false = match cmp_op {
        Less => l.min > r.max,
        LessEqual => l.min >= r.max,
        Greater => l.max < r.min,
        GreaterEqual => l.max <= r.min,
        Equals => l.max < r.min || l.min > r.max,
        NotEquals => l == r,
    };
    if always_true {
        diagnostics.push(Diagnostic {
            location: location.to_string(),
            always_true: true,
        });
    }
    if always_false {
        diagnostics.push(Diagnostic {
            location: location.to_string(),
            always_true: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_value_expression_1() {
        assert_eq!(
            parse_value_expression(
                &Expression::Binary {
                    left: Box::new(Expression::Identifier("a".to_string())),
                    op: "+".to_string(),
                    right: Box::new(Expression::Number(13)),
                },
                &vec![("a".to_string(),
                       NumericalValue::new_value(BoundedValue::Raw(-3), Inclusivity::Inclusive,
                                                 BoundedValue::Raw(-1), Inclusivity::Exclusive))]
                    .into_iter().collect()),
            NumericalValue::new_value(BoundedValue::Raw(10), Inclusivity::Inclusive,
                                      BoundedValue::Raw(12), Inclusivity::Exclusive));
    }

    #[test]
    fn parse_value_expression_2() {
        assert_eq!(
            parse_value_expression(
                &Expression::Binary {
                    left: Box::new(Expression::Identifier("a".to_string())),
                    op: "-".to_string(),
                    right: Box::new(Expression::Number(13)),
                },
                &vec![("a".to_string(),
                       NumericalValue::new_value(BoundedValue::Raw(-3), Inclusivity::Inclusive,
                                                 BoundedValue::Raw(-1), Inclusivity::Exclusive))]
                    .into_iter().collect()),
            NumericalValue::new_value(BoundedValue::Raw(-16), Inclusivity::Inclusive,
                                      BoundedValue::Raw(-14), Inclusivity::Exclusive));

        assert_eq!(
            parse_value_expression(
                &Expression::Binary {
                    left: Box::new(Expression::Number(13)),
                    op: "-".to_string(),
                    right: Box::new(Expression::Identifier("a".to_string())),
                },
                &vec![("a".to_string(),
                       NumericalValue::new_value(BoundedValue::Raw(-3), Inclusivity::Inclusive,
                                                 BoundedValue::Raw(-1), Inclusivity::Exclusive))]
                    .into_iter().collect()),
            NumericalValue::new_value(BoundedValue::Raw(14), Inclusivity::Exclusive,
                                      BoundedValue::Raw(16), Inclusivity::Inclusive));
    }

    #[test]
    fn handle_comparison_1() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        handle_comparison("pos", &Identifier("a".to_string()), "<", &Number(130), &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Min, Inclusivity::Inclusive,
                                        BoundedValue::Raw(130), Inclusivity::Exclusive),
                       fail: Range::new(BoundedValue::Raw(130), Inclusivity::Inclusive,
                                        BoundedValue::Max, Inclusivity::Inclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_2() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        handle_comparison("pos", &Identifier("a".to_string()), "<=", &Number(32), &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Min, Inclusivity::Inclusive,
                                        BoundedValue::Raw(32), Inclusivity::Inclusive),
                       fail: Range::new(BoundedValue::Raw(32), Inclusivity::Exclusive,
                                        BoundedValue::Max, Inclusivity::Inclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_3() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        handle_comparison("pos", &Binary { left: Box::new(Identifier("a".to_string())),
                                    op: "+".to_string(),
                                    right: Box::new(Number(10)), },
                          "<", &Number(32), &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Min, Inclusivity::Inclusive,
                                        BoundedValue::Raw(22), Inclusivity::Exclusive),
                       fail: Range::new(BoundedValue::Raw(22), Inclusivity::Inclusive,
                                        BoundedValue::Max, Inclusivity::Inclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_4() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        handle_comparison("pos", &Binary { left: Box::new(Identifier("a".to_string())),
                                    op: "-".to_string(),
                                    right: Box::new(Number(10)), },
                          "<=", &Number(32), &variables, &mut slices, &mut diagnostics);
        // a - 10 <= 32
        // a <= 42
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Min, Inclusivity::Inclusive,
                                        BoundedValue::Raw(42), Inclusivity::Inclusive),
                       fail: Range::new(BoundedValue::Raw(42), Inclusivity::Exclusive,
                                        BoundedValue::Max, Inclusivity::Inclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_5() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        handle_comparison("pos", &Binary { left: Box::new(Number(10)),
                                    op: "-".to_string(),
                                    right: Box::new(Identifier("a".to_string())), },
                          ">", &Number(32), &variables, &mut slices, &mut diagnostics);
        // 10 - a > 32
        // a < -22
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Min, Inclusivity::Inclusive,
                                        BoundedValue::Raw(-22), Inclusivity::Exclusive),
                       fail: Range::new(BoundedValue::Raw(-22), Inclusivity::Inclusive,
                                        BoundedValue::Max, Inclusivity::Inclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_6() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        // 4 * a <= 32
        // a <= 8
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        handle_comparison("pos", &Binary { left: Box::new(Number(4)),
                                    op: "*".to_string(),
                                    right: Box::new(Identifier("a".to_string())), },
                          "<=", &Number(32), &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Min, Inclusivity::Inclusive,
                                        BoundedValue::Raw(8), Inclusivity::Inclusive),
                       fail: Range::new(BoundedValue::Raw(8), Inclusivity::Exclusive,
                                        BoundedValue::Max, Inclusivity::Inclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_6_2() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        // a * 4 <= 32
        // a <= 8
        handle_comparison("pos", &Binary { left: Box::new(Number(4)),
                                    op: "*".to_string(),
                                    right: Box::new(Identifier("a".to_string())), },
                          "<=", &Number(32), &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Min, Inclusivity::Inclusive,
                                        BoundedValue::Raw(8), Inclusivity::Inclusive),
                       fail: Range::new(BoundedValue::Raw(8), Inclusivity::Exclusive,
                                        BoundedValue::Max, Inclusivity::Inclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_7() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        // 32 / a >= 4
        // a <= 32 / 4
        // a <= 8
        handle_comparison("pos", &Binary { left: Box::new(Number(32)),
                                    op: "/".to_string(),
                                    right: Box::new(Identifier("a".to_string())), },
                          ">=", &Number(4), &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Min, Inclusivity::Inclusive,
                                        BoundedValue::Raw(8), Inclusivity::Inclusive),
                       fail: Range::new(BoundedValue::Raw(8), Inclusivity::Exclusive,
                                        BoundedValue::Max, Inclusivity::Inclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_8() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        // a / 4 <= 32
        // a <= 128
        handle_comparison("pos", &Binary { left: Box::new(Identifier("a".to_string())),
                                    op: "/".to_string(),
                                    right: Box::new(Number(4)), },
                          "<=", &Number(32), &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Min, Inclusivity::Inclusive,
                                        BoundedValue::Raw(128), Inclusivity::Inclusive),
                       fail: Range::new(BoundedValue::Raw(128), Inclusivity::Exclusive,
                                        BoundedValue::Max, Inclusivity::Inclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_9() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        // a == 32
        handle_comparison("pos", &Identifier("a".to_string()),
                          "==", &Number(32), &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::from(BoundedValue::Raw(32)),
                       fail: Range::universe(),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_9_2() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        // a / 4 == 32
        // a == 128
        handle_comparison("pos", &Binary { left: Box::new(Identifier("a".to_string())),
                                    op: "/".to_string(),
                                    right: Box::new(Number(4)), },
                          "==", &Number(32), &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::from(BoundedValue::Raw(128)),
                       fail: Range::universe(),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_10() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        // 32 / a != 4
        // a != 8
        handle_comparison("pos", &Binary { left: Box::new(Number(32)),
                                    op: "/".to_string(),
                                    right: Box::new(Identifier("a".to_string())) },
                          "!=", &Number(4), &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::universe(),
                       fail: Range::from(BoundedValue::Raw(8)),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_11() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        // 32 <= a / 4
        // a >= 128
        handle_comparison("pos", &Number(32),
                          "<=",
                          &Binary { left: Box::new(Identifier("a".to_string())),
                                    op: "/".to_string(),
                                    right: Box::new(Number(4)), },
                          &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Raw(128), Inclusivity::Inclusive,
                                        BoundedValue::Max, Inclusivity::Inclusive),
                       fail: Range::new(BoundedValue::Min, Inclusivity::Inclusive,
                                        BoundedValue::Raw(128), Inclusivity::Exclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_12() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        // 4 != 32 / a
        // a != 8
        handle_comparison("pos", &Number(4),
                          "!=",
                          &Binary { left: Box::new(Number(32)),
                                    op: "/".to_string(),
                                    right: Box::new(Identifier("a".to_string())) },
                          &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::universe(),
                       fail: Range::from(BoundedValue::Raw(8)),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_13() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::universe());
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        // 32 % a < 40
        // a ∈ [0, 40)
        handle_comparison("pos", &Binary { left: Box::new(Number(32)),
                                    op: "%".to_string(),
                                    right: Box::new(Identifier("a".to_string())) },
                          "<", &Number(40), &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Min, Inclusivity::Inclusive,
                                        BoundedValue::Raw(40), Inclusivity::Exclusive),
                       fail: Range::new(BoundedValue::Raw(40), Inclusivity::Inclusive,
                                        BoundedValue::Max, Inclusivity::Inclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_14() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::from(Range::new(
            BoundedValue::Raw(2), Inclusivity::Inclusive,
            BoundedValue::Raw(12), Inclusivity::Inclusive)));
        variables.insert("b".to_string(), NumericalValue::from(Range::new(
            BoundedValue::Raw(0), Inclusivity::Inclusive,
            BoundedValue::Raw(10), Inclusivity::Inclusive)));
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        // a < b
        // [2, 12] < [0, 10]
        // A_T = [2, 10)
        // A_F = [2, 12]
        // B_T = (2, 10]
        // B_F = [0, 10]
        handle_comparison("pos", &Identifier("a".to_string()),
                          "<", &Identifier("b".to_string()),
                          &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Raw(2), Inclusivity::Inclusive,
                                        BoundedValue::Raw(10), Inclusivity::Exclusive),
                       fail: Range::new(BoundedValue::Raw(2), Inclusivity::Inclusive,
                                        BoundedValue::Raw(12), Inclusivity::Inclusive),
                   },
                   VariableValueSlice {
                       name: "b".to_string(),
                       pass: Range::new(BoundedValue::Raw(2), Inclusivity::Exclusive,
                                        BoundedValue::Raw(10), Inclusivity::Inclusive),
                       fail: Range::new(BoundedValue::Raw(0), Inclusivity::Inclusive,
                                        BoundedValue::Raw(10), Inclusivity::Inclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_15() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::from(Range::new(
            BoundedValue::Raw(2), Inclusivity::Inclusive,
            BoundedValue::Raw(12), Inclusivity::Inclusive)));
        variables.insert("b".to_string(), NumericalValue::from(Range::new(
            BoundedValue::Raw(0), Inclusivity::Inclusive,
            BoundedValue::Raw(10), Inclusivity::Inclusive)));
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        // a + 3 < b
        // [2, 12] + 3 < [0, 10]
        // A_T = [2, 7)
        // A_F = [2, 12]
        // B_T = (5, 10]
        // B_F = [0, 10]
        handle_comparison("pos", &Binary { left: Box::new(Identifier("a".to_string())),
                                    op: "+".to_string(),
                                    right: Box::new(Number(3)) },
                          "<", &Identifier("b".to_string()),
                          &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Raw(2), Inclusivity::Inclusive,
                                        BoundedValue::Raw(7), Inclusivity::Exclusive),
                       fail: Range::new(BoundedValue::Raw(2), Inclusivity::Inclusive,
                                        BoundedValue::Raw(12), Inclusivity::Inclusive),
                   },
                   VariableValueSlice {
                       name: "b".to_string(),
                       pass: Range::new(BoundedValue::Raw(5), Inclusivity::Exclusive,
                                        BoundedValue::Raw(10), Inclusivity::Inclusive),
                       fail: Range::new(BoundedValue::Raw(0), Inclusivity::Inclusive,
                                        BoundedValue::Raw(10), Inclusivity::Inclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison_16() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::from(Range::new(
            BoundedValue::Raw(2), Inclusivity::Inclusive,
            BoundedValue::Raw(12), Inclusivity::Inclusive)));
        variables.insert("b".to_string(), NumericalValue::from(Range::new(
            BoundedValue::Raw(0), Inclusivity::Inclusive,
            BoundedValue::Raw(10), Inclusivity::Inclusive)));
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        // b > 3 + a
        // [0, 10] > 3 + [2, 12]
        // A_T = [2, 7)
        // A_F = [2, 12]
        // B_T = (5, 10]
        // B_F = [0, 10]
        handle_comparison("pos", &Identifier("b".to_string()), ">",
                          &Binary { left: Box::new(Number(3)),
                                    op: "+".to_string(),
                                    right: Box::new(Identifier("a".to_string())) },
                          &variables, &mut slices, &mut diagnostics);
        assert_eq!(slices,
                   vec![VariableValueSlice {
                       name: "b".to_string(),
                       pass: Range::new(BoundedValue::Raw(5), Inclusivity::Exclusive,
                                        BoundedValue::Raw(10), Inclusivity::Inclusive),
                       fail: Range::new(BoundedValue::Raw(0), Inclusivity::Inclusive,
                                        BoundedValue::Raw(10), Inclusivity::Inclusive),
                   },
                   VariableValueSlice {
                       name: "a".to_string(),
                       pass: Range::new(BoundedValue::Raw(2), Inclusivity::Inclusive,
                                        BoundedValue::Raw(7), Inclusivity::Exclusive),
                       fail: Range::new(BoundedValue::Raw(2), Inclusivity::Inclusive,
                                        BoundedValue::Raw(12), Inclusivity::Inclusive),
                   }]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn handle_comparison__creates_diagnostics_1() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::from(BoundedValue::Raw(2)));
        variables.insert("b".to_string(), NumericalValue::from(BoundedValue::Raw(10)));
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        handle_comparison("pos", &Identifier("a".to_string()),
                          "<", &Identifier("b".to_string()),
                          &variables, &mut slices, &mut diagnostics);
        assert!(slices.is_empty());
        assert_eq!(diagnostics,
                   vec![Diagnostic {
                       location: "pos".to_string(),
                       message: "Expression is always true".to_string(),
                   }]);
    }

    #[test]
    fn handle_comparison__creates_diagnostics_2() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::from(BoundedValue::Raw(2)));
        variables.insert("b".to_string(), NumericalValue::from(BoundedValue::Raw(10)));
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        handle_comparison("pos", &Identifier("a".to_string()),
                          ">", &Identifier("b".to_string()),
                          &variables, &mut slices, &mut diagnostics);
        assert!(slices.is_empty());
        assert_eq!(diagnostics,
                   vec![Diagnostic {
                       location: "pos".to_string(),
                       message: "Expression is always false".to_string(),
                   }]);
    }

    #[test]
    fn handle_comparison__creates_diagnostics_3() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::new_value(
            BoundedValue::Raw(2), Inclusivity::Inclusive,
            BoundedValue::Raw(7), Inclusivity::Exclusive));
        variables.insert("b".to_string(), NumericalValue::from(BoundedValue::Raw(10)));
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        handle_comparison("pos", &Binary { left: Box::new(Identifier("a".to_string())),
                                           op: "+".to_string(),
                                           right: Box::new(Number(3)) },
                          "<", &Identifier("b".to_string()),
                          &variables, &mut slices, &mut diagnostics);
        assert!(slices.is_empty());
        assert_eq!(diagnostics,
                   vec![Diagnostic {
                       location: "pos".to_string(),
                       message: "Expression is always true".to_string(),
                   }]);
    }

    #[test]
    fn handle_comparison__creates_diagnostics_4() {
        use Expression::*;
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), NumericalValue::new_value(
            BoundedValue::Raw(2), Inclusivity::Inclusive,
            BoundedValue::Raw(7), Inclusivity::Exclusive));
        variables.insert("b".to_string(), NumericalValue::from(BoundedValue::Raw(10)));
        let mut slices = Vec::new();
        let mut diagnostics = Vec::new();
        handle_comparison("pos", &Binary { left: Box::new(Identifier("a".to_string())),
                                           op: "+".to_string(),
                                           right: Box::new(Number(3)) },
                          "==", &Identifier("b".to_string()),
                          &variables, &mut slices, &mut diagnostics);
        assert!(slices.is_empty());
        assert_eq!(diagnostics,
                   vec![Diagnostic {
                       location: "pos".to_string(),
                       message: "Expression is always false".to_string(),
                   }]);
    }

    #[test]
    fn overall_test_1() {
        let values = vec![
            ("a".to_string(),
             NodeValue::VariableDeclaration {
                 declarations: vec![Declaration {
                     identifier: "a".to_string(),
                     initializer: Expression::Other,
                 }]
             }),
            ("b".to_string(),
             NodeValue::VariableDeclaration {
                 declarations: vec![Declaration {
                     identifier: "b".to_string(),
                     initializer: Expression::Other,
                 }]
             }),
            ("c".to_string(),
             NodeValue::Comparison {
                 left: Expression::Identifier("a".to_string()),
                 op: "<".to_string(),
                 right: Expression::Number(13)
             }),
            ("d".to_string(), NodeValue::Other),
            ("e".to_string(),
             NodeValue::Comparison {
                 left: Expression::Identifier("b".to_string()),
                 op: "<=".to_string(),
                 right: Expression::Number(23)
             }),
            ("f".to_string(), NodeValue::Other),
            ("g".to_string(), NodeValue::Other),
        ].into_iter().collect();
        let successors = vec![
            ("a".to_string(), vec![Successor { key: "b".to_string(), value: -1 }]),
            ("b".to_string(), vec![Successor { key: "c".to_string(), value: -1 }]),
            ("c".to_string(), vec![Successor { key: "d".to_string(), value: 1 },
                                   Successor { key: "e".to_string(), value: 0 }]),
            ("d".to_string(), vec![Successor { key: "g".to_string(), value: -1 }]),
            ("e".to_string(), vec![Successor { key: "f".to_string(), value: 1 },
                                   Successor { key: "g".to_string(), value: 0 }]),
            ("f".to_string(), vec![Successor { key: "g".to_string(), value: -1 }]),
            ("g".to_string(), vec![]),
        ].into_iter().collect();
        let graph = Graph::new(values, successors, "a".to_string());
        let mut variables = HashMap::new();
        let mut history = HashMap::new();
        let mut diagnostics = Vec::new();
        numerical_value_analysis(&graph, "a", &mut variables, &mut history, &mut diagnostics);
        assert_eq!(format!("{:?}", history["d"]["a"]), "[-inf, 13)");
        assert_eq!(format!("{:?}", history["d"]["b"]), "[-inf, inf]");
        assert!(diagnostics.is_empty());
    }
}
