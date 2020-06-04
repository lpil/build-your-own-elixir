//!  This module implements the functionality described in
//!  ["Strictly Pretty" (2000) by Christian Lindig][0], with a few
//!  extensions.
//!
//!  [0]: http://citeseerx.ist.psu.edu/viewdoc/summary?doi=10.1.1.34.2200

use im::vector::Vector;

use std::{borrow::Cow, marker::PhantomData};

pub trait Documentable<'a> {
    fn to_doc(self) -> Document<'a>;
}

impl Documentable<'static> for &str {
    fn to_doc(self) -> Document<'static> {
        Document::Text(self.to_string().into())
    }
}

impl Documentable<'static> for String {
    fn to_doc(self) -> Document<'static> {
        Document::Text(self.into())
    }
}

impl Documentable<'static> for isize {
    fn to_doc(self) -> Document<'static> {
        Document::Text(format!("{}", self).into())
    }
}

impl Documentable<'static> for i64 {
    fn to_doc(self) -> Document<'static> {
        Document::Text(format!("{}", self).into())
    }
}

impl Documentable<'static> for usize {
    fn to_doc(self) -> Document<'static> {
        Document::Text(format!("{}", self).into())
    }
}

impl Documentable<'static> for f64 {
    fn to_doc(self) -> Document<'static> {
        Document::Text(format!("{:?}", self).into())
    }
}

impl Documentable<'static> for u64 {
    fn to_doc(self) -> Document<'static> {
        Document::Text(format!("{:?}", self).into())
    }
}

impl Documentable<'static> for Document<'static> {
    fn to_doc(self) -> Document<'static> {
        self
    }
}

impl Documentable<'static> for Vec<Document<'static>> {
    fn to_doc(self) -> Document<'static> {
        concat(self.into_iter())
    }
}

impl<D: Documentable<'static>> Documentable<'static> for Option<D> {
    fn to_doc(self) -> Document<'static> {
        match self {
            Some(d) => d.to_doc(),
            None => Document::Nil,
        }
    }
}

pub fn concat(mut docs: impl Iterator<Item = Document<'static>>) -> Document<'static> {
    let init = docs.next().unwrap_or_else(|| nil());
    docs.fold(init, |acc, doc| {
        Document::Cons(Box::new(acc), Box::new(doc))
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Document<'a> {
    /// Returns a document entity used to represent nothingness
    Nil,

    /// A mandatory linebreak
    Line(usize),

    /// Forces contained groups to break
    ForceBreak,

    /// Renders `broken` if group is broken, `unbroken` otherwise
    // Notice the phantom here. This allows us to put a generic lifetime for
    // Document.
    //
    // If you see it in a PR, then something wen wrong.
    Break {
        broken: Cow<'a, str>,
        unbroken: Cow<'a, str>,
        phantom: PhantomData<&'a ()>,
    },

    /// Join 2 documents together
    Cons(Box<Document<'a>>, Box<Document<'a>>),

    /// Nests the given document by the given indent
    Nest(isize, Box<Document<'a>>),

    /// Nests the given document to the current cursor position
    NestCurrent(Box<Document<'a>>),

    /// Nests the given document to the current cursor position
    Group(Box<Document<'a>>),

    /// A string to render
    Text(Cow<'a, str>),
}

#[derive(Debug, Clone)]
enum Mode {
    Broken,
    Unbroken,
}

fn fits(mut limit: isize, mut docs: Vector<(isize, Mode, Document)>) -> bool {
    loop {
        if limit < 0 {
            return false;
        };

        let (indent, mode, document) = match docs.pop_front() {
            Some(x) => x,
            None => return true,
        };

        match document {
            Document::Nil => (),

            Document::Line(_) => return true,

            Document::ForceBreak => return false,

            Document::Nest(i, doc) => docs.push_front((i + indent, mode, *doc)),

            Document::NestCurrent(doc) => docs.push_front((indent, mode, *doc)),

            Document::Group(doc) => docs.push_front((indent, Mode::Unbroken, *doc)),

            Document::Text(s) => limit -= s.len() as isize,

            Document::Break { unbroken, .. } => match mode {
                Mode::Broken => return true,
                Mode::Unbroken => limit -= unbroken.len() as isize,
            },

            Document::Cons(left, right) => {
                docs.push_front((indent, mode.clone(), *right));
                docs.push_front((indent, mode, *left));
            }
        }
    }
}

pub fn format(limit: isize, doc: Document) -> String {
    let mut buffer = String::new();
    fmt(
        &mut buffer,
        limit,
        0,
        vector![(0, Mode::Unbroken, Document::Group(Box::new(doc)))],
    );
    buffer
}

fn fmt(b: &mut String, limit: isize, mut width: isize, mut docs: Vector<(isize, Mode, Document)>) {
    while let Some((indent, mode, document)) = docs.pop_front() {
        match document {
            Document::Nil | Document::ForceBreak => (),

            Document::Line(i) => {
                for _ in 0..i {
                    b.push_str("\n");
                }
                b.push_str(" ".repeat(indent as usize).as_str());
                width = indent;
            }

            Document::Break {
                broken, unbroken, ..
            } => {
                width = match mode {
                    Mode::Unbroken => {
                        b.push_str(unbroken.as_ref());
                        width + unbroken.len() as isize
                    }
                    Mode::Broken => {
                        b.push_str(broken.as_ref());
                        b.push_str("\n");
                        b.push_str(" ".repeat(indent as usize).as_str());
                        indent as isize
                    }
                };
            }

            Document::Text(s) => {
                width += s.len() as isize;
                b.push_str(s.as_ref());
            }

            Document::Cons(left, right) => {
                docs.push_front((indent, mode.clone(), *right));
                docs.push_front((indent, mode, *left));
            }

            Document::Nest(i, doc) => {
                docs.push_front((indent + i, mode, *doc));
            }

            Document::NestCurrent(doc) => {
                docs.push_front((width, mode, *doc));
            }

            Document::Group(doc) => {
                docs.push_front((indent, Mode::Unbroken, (*doc).clone()));
                if !fits(limit - width, docs.clone()) {
                    docs[0] = (indent, Mode::Broken, (*doc).clone());
                }
            }
        }
    }
}

#[test]
fn fits_test() {
    use self::Document::*;
    use self::Mode::*;

    // Negative limits never fit
    assert!(!fits(-1, vector![]));

    // If no more documents it always fits
    assert!(fits(0, vector![]));

    // ForceBreak never fits
    assert!(!fits(100, vector![(0, Unbroken, ForceBreak)]));
    assert!(!fits(100, vector![(0, Broken, ForceBreak)]));

    // Break in Broken fits always
    assert!(fits(
        1,
        vector![(
            0,
            Broken,
            Break {
                broken: "12".to_string().into(),
                unbroken: "".to_string().into(),
                phantom: PhantomData,
            }
        )]
    ));

    // Break in Unbroken mode fits if `unbroken` fits
    assert!(fits(
        3,
        vector![(
            0,
            Unbroken,
            Break {
                broken: "".to_string().into(),
                unbroken: "123".to_string().into(),
                phantom: PhantomData,
            }
        )]
    ));
    assert!(!fits(
        2,
        vector![(
            0,
            Unbroken,
            Break {
                broken: "".to_string().into(),
                unbroken: "123".to_string().into(),
                phantom: PhantomData,
            }
        )]
    ));

    // Line always fits
    assert!(fits(0, vector![(0, Broken, Line(100))]));
    assert!(fits(0, vector![(0, Unbroken, Line(100))]));

    // String fits if smaller than limit
    assert!(fits(
        5,
        vector![(0, Broken, Text("Hello".to_string().into()))]
    ));
    assert!(fits(
        5,
        vector![(0, Unbroken, Text("Hello".to_string().into()))]
    ));
    assert!(!fits(
        4,
        vector![(0, Broken, Text("Hello".to_string().into()))]
    ));
    assert!(!fits(
        4,
        vector![(0, Unbroken, Text("Hello".to_string().into()))]
    ));

    // Cons fits if combined smaller than limit
    assert!(fits(
        2,
        vector![(
            0,
            Broken,
            Cons(
                Box::new(Text("1".to_string().into())),
                Box::new(Text("2".to_string().into()))
            )
        )]
    ));
    assert!(fits(
        2,
        vector![(
            0,
            Unbroken,
            Cons(
                Box::new(Text("1".to_string().into())),
                Box::new(Text("2".to_string().into()))
            )
        )]
    ));
    assert!(!fits(
        1,
        vector![(
            0,
            Broken,
            Cons(
                Box::new(Text("1".to_string().into())),
                Box::new(Text("2".to_string().into()))
            )
        )]
    ));
    assert!(!fits(
        1,
        vector![(
            0,
            Unbroken,
            Cons(
                Box::new(Text("1".to_string().into())),
                Box::new(Text("2".to_string().into()))
            )
        )]
    ));

    // Nest fits if combined smaller than limit
    assert!(fits(
        2,
        vector![(0, Broken, Nest(1, Box::new(Text("12".to_string().into()))))]
    ));
    assert!(fits(
        2,
        vector![(
            0,
            Unbroken,
            Nest(1, Box::new(Text("12".to_string().into())))
        )]
    ));
    assert!(!fits(
        1,
        vector![(0, Broken, Nest(1, Box::new(Text("12".to_string().into()))))]
    ));
    assert!(!fits(
        1,
        vector![(
            0,
            Unbroken,
            Nest(1, Box::new(Text("12".to_string().into())))
        )]
    ));

    // Nest fits if combined smaller than limit
    assert!(fits(
        2,
        vector![(
            0,
            Broken,
            NestCurrent(Box::new(Text("12".to_string().into())))
        )]
    ));
    assert!(fits(
        2,
        vector![(
            0,
            Unbroken,
            NestCurrent(Box::new(Text("12".to_string().into())))
        )]
    ));
    assert!(!fits(
        1,
        vector![(
            0,
            Broken,
            NestCurrent(Box::new(Text("12".to_string().into())))
        )]
    ));
    assert!(!fits(
        1,
        vector![(
            0,
            Unbroken,
            NestCurrent(Box::new(Text("12".to_string().into())))
        )]
    ));
}

#[test]
fn format_test() {
    use self::Document::*;

    let doc = Text("Hi".to_string().into());
    assert_eq!("Hi".to_string(), format(10, doc));

    let doc = Cons(
        Box::new(Text("Hi".to_string().into())),
        Box::new(Text(", world!".to_string().into())),
    );
    assert_eq!("Hi, world!".to_string(), format(10, doc));

    let doc = Nil;
    assert_eq!("".to_string(), format(10, doc));

    let doc = Break {
        broken: "broken".to_string().into(),
        unbroken: "unbroken".to_string().into(),
        phantom: PhantomData,
    };
    assert_eq!("unbroken".to_string(), format(10, doc));

    let doc = Break {
        broken: "broken".to_string().into(),
        unbroken: "unbroken".to_string().into(),
        phantom: PhantomData,
    };
    assert_eq!("broken\n".to_string(), format(5, doc));

    let doc = Nest(
        2,
        Box::new(Cons(
            Box::new(Text("1".to_string().into())),
            Box::new(Cons(
                Box::new(Line(1)),
                Box::new(Text("2".to_string().into())),
            )),
        )),
    );
    assert_eq!("1\n  2".to_string(), format(1, doc));

    let doc = Cons(
        Box::new(Text("111".to_string().into())),
        Box::new(NestCurrent(Box::new(Cons(
            Box::new(Line(1)),
            Box::new(Text("2".to_string().into())),
        )))),
    );
    assert_eq!("111\n   2".to_string(), format(1, doc));

    let doc = Cons(
        Box::new(ForceBreak),
        Box::new(Break {
            broken: "broken".to_string().into(),
            unbroken: "unbroken".to_string().into(),
            phantom: PhantomData,
        }),
    );
    assert_eq!("broken\n".to_string(), format(100, doc));
}

pub fn nil() -> Document<'static> {
    Document::Nil
}

pub fn line() -> Document<'static> {
    Document::Line(1)
}

pub fn lines(i: usize) -> Document<'static> {
    Document::Line(i)
}

pub fn force_break() -> Document<'static> {
    Document::ForceBreak
}

pub fn break_(broken: &str, unbroken: &str) -> Document<'static> {
    Document::Break {
        broken: broken.to_string().into(),
        unbroken: unbroken.to_string().into(),
        phantom: PhantomData,
    }
}

pub fn delim(d: &str) -> Document<'static> {
    Document::Break {
        broken: d.to_string().into(),
        unbroken: format!("{} ", d).into(),
        phantom: PhantomData,
    }
}

impl Document<'static> {
    pub fn group(self) -> Document<'static> {
        Document::Group(Box::new(self))
    }

    pub fn nest(self, indent: isize) -> Document<'static> {
        Document::Nest(indent, Box::new(self))
    }

    pub fn nest_current(self) -> Document<'static> {
        Document::NestCurrent(Box::new(self))
    }

    pub fn append(self, x: impl Documentable<'static>) -> Document<'static> {
        Document::Cons(Box::new(self), Box::new(x.to_doc()))
    }

    pub fn format(self, limit: isize) -> String {
        format(limit, self)
    }

    pub fn surround(
        self,
        open: impl Documentable<'static>,
        closed: impl Documentable<'static>,
    ) -> Document<'static> {
        open.to_doc().append(self).append(closed)
    }
}
