#![cfg_attr(doc,doc = document_features::document_features!())]

use ftml_uris::{Id, SymbolUri};
use std::collections::BTreeMap;

#[must_use]
pub fn to_html(source: &str) -> String {
    let mut options = HTMLOptions::default();
    markdown_lite::html::to_html_with_options(source, &mut options)
}

#[must_use]
pub fn to_html_with_math(
    source: &str,
    inline: impl Fn(&str, &mut String),
    block: impl Fn(&str, &mut String),
) -> String {
    let mut options = HTMLOptions {
        inline_math: Some(&inline as _),
        block_math: Some(&block as _),
        ..Default::default()
    };
    markdown_lite::html::to_html_with_options(source, &mut options)
}

#[must_use]
pub fn to_latex(source: &str) -> String {
    let mut options = LaTeXOptions::default();
    markdown_lite::latex::to_latex_with_options(source, &mut options)
}

#[derive(Default)]
struct HTMLOptions<'a> {
    symbols: BTreeMap<Id, SymbolUri>,
    in_definition: bool,
    inline_math: Option<&'a dyn Fn(&str, &mut String)>,
    block_math: Option<&'a dyn Fn(&str, &mut String)>,
}
impl HTMLOptions<'_> {
    fn symbol(&mut self, args: &str, out: &mut String) {
        use std::fmt::Write;
        let (name, text) = if let Some((a, b)) = args.split_once(',') {
            (a, Some(b))
        } else {
            (args, None)
        };
        let _ = if let Some(sym) = self
            .symbols
            .iter()
            .find_map(|(k, v)| if k.as_ref() == name { Some(v) } else { None })
        {
            if let Some(text) = text {
                let _ = write!(
                    out,
                    "<span data-ftml-comp data-ftml-term=\"OMID\" data-ftml-head=\"{sym}\">", //{text}</span>"
                );
                markdown_lite::html::inline_html_with_options(text, out, self);
                out.write_str("</span>")
            } else {
                write!(
                    out,
                    "<span data-ftml-comp data-ftml-term=\"OMID\" data-ftml-head=\"{sym}\">{}</span>",
                    sym.name()
                )
            }
        } else {
            write!(
                out,
                "<span style=\"color:red\">ERROR: Symbol &quot;{name}&quot; not found</span>"
            )
        };
    }

    fn definiendum(&mut self, args: &str, out: &mut String) {
        use std::fmt::Write;
        let (name, text) = if let Some((a, b)) = args.split_once(',') {
            (a, Some(b))
        } else {
            (args, None)
        };
        let _ = if let Some(sym) = self
            .symbols
            .iter()
            .find_map(|(k, v)| if k.as_ref() == name { Some(v) } else { None })
        {
            if let Some(text) = text {
                let _ = write!(
                    out,
                    "<span data-ftml-definiendum=\"{sym}\">", //{text}</span>"
                );
                markdown_lite::html::inline_html_with_options(text, out, self);
                out.write_str("</span>")
            } else {
                write!(
                    out,
                    "<span data-ftml-definiendum=\"{sym}\">{}</span>",
                    sym.name()
                )
            }
        } else {
            write!(
                out,
                "<span style=\"color:red\">ERROR: Symbol &quot;{name}&quot; not found</span>"
            )
        };
    }

    fn inline_def(&mut self, args: &str, out: &mut String) {
        use std::fmt::Write;
        let old = std::mem::replace(&mut self.in_definition, true);
        let _ = write!(
            out,
            "<span data-ftml-definition data-ftml-inline=\"true\">", //{text}</span>"
        );
        markdown_lite::html::inline_html_with_options(args, out, self);
        let _ = out.write_str("</span>");
        self.in_definition = old;
    }

    fn definition(&mut self, args: &str, out: &mut String) {
        use std::fmt::Write;
        let title = {
            if let Some(i) = args.find("title") {
                let mut rest = (&args[i + "title".len()..]);
                if rest.starts_with('=') {
                    rest = (&rest[1..]);
                    if rest.starts_with('"') {
                        rest = (&rest[1..]);
                        if let Some(end) = rest.find('"') {
                            (&rest[..end]).trim()
                        } else {
                            ""
                        }
                    } else if rest.starts_with('\'') {
                        rest = (&rest[1..]);
                        if let Some(end) = rest.find('\'') {
                            (&rest[..end]).trim()
                        } else {
                            ""
                        }
                    } else {
                        ""
                    }
                } else {
                    ""
                }
            } else {
                ""
            }
        };
        self.in_definition = true;
        let _ = write!(
            out,
            "<div data-ftml-definition data-ftml-inline=\"false\">\n<span data-ftml-title>", //{text}</span>"
        );
        markdown_lite::html::inline_html_with_options(title, out, self);
        let _ = out.write_str("</span>");
    }
}

impl markdown_lite::html::Options for HTMLOptions<'_> {
    fn frontmatter(&mut self, fm: serde_value::Value, out: &mut String) {
        use std::fmt::Write;
        match &fm {
            serde_value::Value::Map(m) => {
                self.symbols = get_symbols(m);
                let _ = write!(
                    out,
                    "<pre style=\"background-color:antiquewhite\">{:#?}</pre>",
                    self.symbols
                );
            }
            o => {
                let _ = write!(out, "<pre style=\"background-color:red\">{o:#?}</pre>");
            }
        }
    }

    fn inline_custom(&mut self, label: &str, arguments: &str, out: &mut String) -> bool {
        match label {
            "sym" => self.symbol(arguments, out),
            "definition" => self.inline_def(arguments, out),
            "def" if self.in_definition => self.definiendum(arguments, out),
            _ => return false,
        }
        true
    }

    fn custom_block_start(&mut self, label: &str, arguments: &str, out: &mut String) -> bool {
        match label {
            "definition" => self.definition(arguments, out),
            _ => return false,
        }
        true
    }

    fn custom_block_end(&mut self, label: &str, out: &mut String) -> bool {
        match label {
            "definition" => {
                out.push_str("</div>");
                true
            }
            _ => false,
        }
    }

    fn inline_math(&mut self, math: &str, out: &mut String) -> bool {
        self.inline_math.is_some_and(|f| {
            f(math, out);
            true
        })
    }

    fn block_math(&mut self, math: &str, out: &mut String) -> bool {
        self.block_math.is_some_and(|f| {
            f(math, out);
            true
        })
    }
}

#[derive(Default)]
struct LaTeXOptions {
    symbols: BTreeMap<Id, SymbolUri>,
    in_definition: bool,
}

impl LaTeXOptions {
    fn symbol(&mut self, args: &str, out: &mut String) {
        use std::fmt::Write;
        let (name, text) = if let Some((a, b)) = args.split_once(',') {
            (a, Some(b))
        } else {
            (args, None)
        };
        if let Some(text) = text {
            let _ = write!(
                out,
                "\\sr{{{name}}}{{", //{text}</span>"
            );
            markdown_lite::latex::inline_latex_with_options(text, out, self);
            out.push('}');
        } else {
            let _ = write!(out, "\\sn{{{name}}}");
        }
    }

    fn definiendum(&mut self, args: &str, out: &mut String) {
        use std::fmt::Write;
        let (name, text) = if let Some((a, b)) = args.split_once(',') {
            (a, Some(b))
        } else {
            (args, None)
        };
        if let Some(text) = text {
            let _ = write!(
                out,
                "\\definiendum{{{name}}}{{", //{text}</span>"
            );
            markdown_lite::latex::inline_latex_with_options(text, out, self);
            out.push('}');
        } else {
            let _ = write!(out, "\\definame{{{name}}}");
        }
    }

    fn inline_def(&mut self, args: &str, out: &mut String) {
        let old = std::mem::replace(&mut self.in_definition, true);
        out.push_str("\\inlinedef{");
        markdown_lite::latex::inline_latex_with_options(args, out, self);
        out.push('}');
        self.in_definition = old;
    }

    fn definition(&mut self, args: &str, out: &mut String) {
        use std::fmt::Write;
        let title = {
            if let Some(i) = args.find("title") {
                let mut rest = &args[i + "title".len()..];
                if rest.starts_with('=') {
                    rest = &rest[1..];
                    if rest.starts_with('"') {
                        rest = &rest[1..];
                        if let Some(end) = rest.find('"') {
                            (&rest[..end]).trim()
                        } else {
                            ""
                        }
                    } else if rest.starts_with('\'') {
                        rest = &rest[1..];
                        if let Some(end) = rest.find('\'') {
                            (&rest[..end]).trim()
                        } else {
                            ""
                        }
                    } else {
                        ""
                    }
                } else {
                    ""
                }
            } else {
                ""
            }
        };
        self.in_definition = true;
        out.push_str("\\begin{sdefinition}[title={");
        markdown_lite::latex::inline_latex_with_options(title, out, self);
        let _ = out.write_str("}]");
    }
}

impl markdown_lite::latex::Options for LaTeXOptions {
    fn inline_custom(&mut self, label: &str, arguments: &str, out: &mut String) -> bool {
        match label {
            "sym" => self.symbol(arguments, out),
            "definition" => self.inline_def(arguments, out),
            "def" if self.in_definition => self.definiendum(arguments, out),
            _ => return false,
        }
        true
    }

    fn custom_block_start(&mut self, label: &str, arguments: &str, out: &mut String) -> bool {
        match label {
            "definition" => self.definition(arguments, out),
            _ => return false,
        }
        true
    }
    fn custom_block_end(&mut self, label: &str, out: &mut String) -> bool {
        match label {
            "definition" => {
                out.push_str("\n\\end{sdefinition}");
                true
            }
            _ => false,
        }
    }
}

fn get_symbols(map: &BTreeMap<serde_value::Value, serde_value::Value>) -> BTreeMap<Id, SymbolUri> {
    let mut ret = BTreeMap::new();
    if let Some(symbols) = map.iter().find_map(|(k, v)| match (k, v) {
        (serde_value::Value::String(s), serde_value::Value::Map(v)) if s == "symbols" => Some(v),
        _ => None,
    }) {
        for p in symbols {
            if let (serde_value::Value::String(k), serde_value::Value::String(v)) = p
                && let Ok(id) = k.parse()
                && let Ok(uri) = v.parse()
            {
                ret.insert(id, uri);
            }
        }
    }
    ret
}
