fn parse_int(s: &str) -> (u32, &str) {
    for (i, c) in s.chars().enumerate() {
        if !c.is_ascii_digit() {
            return (s[..i].parse().unwrap(), &s[i..]);
        }
    }
    (s.parse().unwrap(), "")
}

fn parse_brackets(s: &str) -> (Vec<u32>, &str) {
    let mut lst = Vec::new();
    let mut rest = s;

    while !rest.is_empty() {
        if rest.starts_with(',') {
            rest = &rest[1..];
            continue;
        }
        if let Some(stripped) = rest.strip_prefix(']') {
            return (lst, stripped);
        }
        let (a, new_rest) = parse_int(rest);
        if new_rest.starts_with(',') || new_rest.starts_with(']') {
            lst.push(a);
        } else if let Some(stripped) = new_rest.strip_prefix('-') {
            let (b, new_rest) = parse_int(stripped);
            lst.extend(a..=b);
            rest = new_rest;
            continue;
        }
        rest = new_rest;
    }
    (lst, rest)
}

fn parse_node(s: &str) -> (Vec<String>, &str) {
    for (i, c) in s.chars().enumerate() {
        if c == ',' {
            return (vec![s[..i].to_string()], &s[i + 1..]);
        }
        if c == '[' {
            let (brackets, rest) = parse_brackets(&s[i + 1..]);
            let mut nodes = Vec::new();
            for z in brackets {
                nodes.push(format!("{}{}", &s[..i], z));
            }
            let new_rest = if !rest.is_empty() && rest.starts_with(',') {
                &rest[1..]
            } else {
                rest
            };
            return (nodes, new_rest);
        }
    }
    (vec![s.to_string()], "")
}

// solution is inspired by https://gist.github.com/ebirn/cf52876120648d7d85501fcbf185ff07
pub fn parse_list(s: &str) -> Vec<String> {
    let mut nodes = Vec::new();
    let mut rest = s;

    while !rest.is_empty() {
        let (v, new_rest) = parse_node(rest);
        nodes.extend(v);
        rest = new_rest;
    }

    nodes
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_list() {
        assert_eq!(
            parse_list("c1-[0-1],c2-[2-3]"),
            vec!["c1-0", "c1-1", "c2-2", "c2-3"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c1-0,c2-0"),
            vec!["c1-0", "c2-0"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c1-0,c2-1"),
            vec!["c1-0", "c2-1"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c2-1"),
            vec!["c2-1"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c2-[1,3,5]"),
            vec!["c2-1", "c2-3", "c2-5"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c2-[1-3,5]"),
            vec!["c2-1", "c2-2", "c2-3", "c2-5"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c3-[1-3,5,9-12]"),
            vec!["c3-1", "c3-2", "c3-3", "c3-5", "c3-9", "c3-10", "c3-11", "c3-12"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c3-[5,9-12]"),
            vec!["c3-5", "c3-9", "c3-10", "c3-11", "c3-12"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c3-[5,9],c5-[15-19]"),
            vec!["c3-5", "c3-9", "c5-15", "c5-16", "c5-17", "c5-18", "c5-19"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c3-[5,9],c5-[15,17]"),
            vec!["c3-5", "c3-9", "c5-15", "c5-17"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c3-5,c7-[15,17]"),
            vec!["c3-5", "c7-15", "c7-17"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c3-[5,9],c8-175"),
            vec!["c3-5", "c3-9", "c8-175"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c1-20"),
            vec!["c1-20"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c1-34,c2-[3,21]"),
            vec!["c1-34", "c2-3", "c2-21"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c1-[34,37-38,41]"),
            vec!["c1-34", "c1-37", "c1-38", "c1-41"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c5-54,c11-30"),
            vec!["c5-54", "c11-30"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );

        assert_eq!(
            parse_list("c2-[1,3-5]"),
            vec!["c2-1", "c2-3", "c2-4", "c2-5"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );
    }
}
