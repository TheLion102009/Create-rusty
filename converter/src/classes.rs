//! Create's class hierarchy, read from its Java sources: which classes extend or implement
//! which, so the converter can ask what a block's Java class *is* (`IWrenchable`, `IRotate`)
//! the way Create's code does with `instanceof`.
//!
//! A declaration reader, not a Java parser: it finds `class`, `interface`, `enum` and `record`
//! headers and their `extends` / `implements` lists, by simple name.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Default)]
pub struct Classes {
    /// Each type's direct supertypes, by simple name.
    parents: BTreeMap<String, Vec<String>>,
}

impl Classes {
    /// Every type declared in the `.java` files under `dir`.
    pub fn read(dir: &Path) -> Classes {
        let mut classes = Classes::default();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&d) else { continue };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "java")
                    && let Ok(src) = std::fs::read_to_string(&p)
                {
                    classes.add_source(&src);
                }
            }
        }
        classes
    }

    pub fn add_source(&mut self, src: &str) {
        let src = without_comments(src);
        for keyword in ["class ", "interface ", "enum ", "record "] {
            let mut rest = src.as_str();
            while let Some(at) = rest.find(keyword) {
                let before = rest[..at].chars().last();
                rest = &rest[at + keyword.len()..];
                // `class` as a word, not inside an identifier (`subclass `) or `X.class`.
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                    continue;
                }
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if name.is_empty() || !name.starts_with(|c: char| c.is_uppercase()) {
                    continue;
                }
                let header = rest[name.len()..].split('{').next().unwrap_or("");
                let header = strip_generics(header);
                // A record's components come first: `record R(int a) implements X`.
                let header = match header.find(')') {
                    Some(i) if keyword == "record " => &header[i + 1..],
                    _ => header.as_str(),
                };
                let mut parents = Vec::new();
                for clause in ["extends", "implements"] {
                    if let Some(i) = find_word(header, clause) {
                        let list = &header[i + clause.len()..];
                        let end = find_word(list, "implements").unwrap_or(list.len());
                        for t in list[..end].split(',') {
                            let t = t.trim().rsplit('.').next().unwrap_or("").trim();
                            if !t.is_empty() {
                                parents.push(t.to_owned());
                            }
                        }
                    }
                }
                self.parents.entry(name).or_default().extend(parents);
            }
        }
    }

    /// Whether `class` is, or extends or implements, `ancestor`, as `instanceof` sees it.
    pub fn is(&self, class: &str, ancestor: &str) -> bool {
        let mut seen = BTreeSet::new();
        let mut stack = vec![class.to_owned()];
        while let Some(c) = stack.pop() {
            if c == ancestor {
                return true;
            }
            if seen.insert(c.clone())
                && let Some(ps) = self.parents.get(&c)
            {
                stack.extend(ps.iter().cloned());
            }
        }
        false
    }
}

fn find_word(s: &str, word: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(i) = s[from..].find(word) {
        let at = from + i;
        let before = s[..at].chars().last();
        let after = s[at + word.len()..].chars().next();
        let boundary = |c: Option<char>| c.is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        if boundary(before) && boundary(after) {
            return Some(at);
        }
        from = at + word.len();
    }
    None
}

/// `Foo<Bar<Baz>, Qux>` as `Foo`.
fn strip_generics(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

/// Java source without `//` and `/* */` comments or string literals' contents.
fn without_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut last = ' ';
                for c in chars.by_ref() {
                    if last == '*' && c == '/' {
                        break;
                    }
                    last = c;
                }
                out.push(' ');
            }
            '"' => {
                out.push('"');
                let mut escaped = false;
                for c in chars.by_ref() {
                    if c == '"' && !escaped {
                        break;
                    }
                    escaped = c == '\\' && !escaped;
                }
                out.push('"');
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_extends_and_implements_through_generics() {
        let mut c = Classes::default();
        c.add_source("public interface IWrenchable {}");
        c.add_source("public interface IRotate extends IWrenchable { }");
        c.add_source(
            "public class KineticBlock extends Block implements IRotate, ProperWaterloggedBlock {}",
        );
        c.add_source(
            "// class Fake extends IWrenchable\npublic class GearboxBlock extends RotatedPillarKineticBlock implements IBE<GearboxBlockEntity, Map<A, B>> {}",
        );
        c.add_source("public abstract class RotatedPillarKineticBlock extends KineticBlock {}");
        c.add_source("public class Plain extends Block { String s = \"class X extends IWrenchable\"; }");
        assert!(c.is("GearboxBlock", "IWrenchable"));
        assert!(c.is("GearboxBlock", "IBE"));
        assert!(!c.is("Plain", "IWrenchable"));
        assert!(!c.is("Fake", "IWrenchable"));
    }

    #[test]
    fn records_and_qualified_names() {
        let mut c = Classes::default();
        c.add_source("public record Thing(int a, Map<String, Integer> b) implements com.x.Marker {}");
        assert!(c.is("Thing", "Marker"));
    }
}
