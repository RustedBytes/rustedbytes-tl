use crate::Node;

/// A single query selector node
#[derive(Debug, Clone)]
pub enum Selector<'a, const MAX_SELECTOR_NODES: usize = 0> {
    /// Tag selector: foo
    Tag(&'a [u8]),
    /// ID selector: #foo
    Id(&'a [u8]),
    /// Class selector: .foo
    Class(&'a [u8]),
    /// All selector: *
    All,
    /// And combinator: .foo.bar
    #[cfg(feature = "std")]
    And(
        Box<Selector<'a, MAX_SELECTOR_NODES>>,
        Box<Selector<'a, MAX_SELECTOR_NODES>>,
    ),
    /// Or combinator: .foo, .bar
    #[cfg(feature = "std")]
    Or(
        Box<Selector<'a, MAX_SELECTOR_NODES>>,
        Box<Selector<'a, MAX_SELECTOR_NODES>>,
    ),
    /// Descendant combinator: .foo .bar
    #[cfg(feature = "std")]
    Descendant(
        Box<Selector<'a, MAX_SELECTOR_NODES>>,
        Box<Selector<'a, MAX_SELECTOR_NODES>>,
    ),
    /// Parent combinator: .foo > .bar
    #[cfg(feature = "std")]
    Parent(
        Box<Selector<'a, MAX_SELECTOR_NODES>>,
        Box<Selector<'a, MAX_SELECTOR_NODES>>,
    ),
    /// Adjacent element sibling combinator: a + b.
    #[cfg(feature = "std")]
    Adjacent(Box<Self>, Box<Self>),
    /// General element sibling combinator: a ~ b.
    #[cfg(feature = "std")]
    Sibling(Box<Self>, Box<Self>),
    /// Negation pseudo-class: :not(selector).
    #[cfg(feature = "std")]
    Not(Box<Self>),
    /// Descendant existence pseudo-class: :has(selector).
    /// Relative selectors such as :has(> a) are not supported.
    #[cfg(feature = "std")]
    Has(Box<Self>),
    /// Element position pseudo-class: :nth-child(an+b), using one-based indices.
    #[cfg(feature = "std")]
    NthChild(i32, i32),
    /// Attribute: \[foo\]
    Attribute(&'a [u8]),
    /// Attribute with value: [foo=bar]
    AttributeValue(&'a [u8], &'a [u8]),
    /// Attribute with whitespace-separated list of values that contains a value: [foo~=bar]
    AttributeValueWhitespacedContains(&'a [u8], &'a [u8]),
    /// Attribute with value that starts with: [foo^=bar]
    AttributeValueStartsWith(&'a [u8], &'a [u8]),
    /// Attribute with value that ends with: [foo$=bar]
    AttributeValueEndsWith(&'a [u8], &'a [u8]),
    /// Attribute with value that contains: [foo*=bar]
    AttributeValueSubstring(&'a [u8], &'a [u8]),
}

impl<'a, const MAX_SELECTOR_NODES: usize> Selector<'a, MAX_SELECTOR_NODES> {
    /// Checks a node without tree context. Structural selectors require `query_selector`.
    pub fn matches<'b>(&self, node: &Node<'b>) -> bool {
        match self {
            Self::Tag(tag) => node
                .as_tag()
                .is_some_and(|t| t._name.as_bytes().eq_ignore_ascii_case(tag)),
            Self::Id(id) => node
                .as_tag()
                .is_some_and(|t| t._attributes.id == Some((*id).into())),
            Self::Class(class) => node
                .as_tag()
                .is_some_and(|t| t._attributes.is_class_member(*class)),
            #[cfg(feature = "std")]
            Self::And(a, b) => a.matches(node) && b.matches(node),
            #[cfg(feature = "std")]
            Self::Or(a, b) => a.matches(node) || b.matches(node),
            #[cfg(feature = "std")]
            Self::Not(a) => node.as_tag().is_some() && !a.matches(node),
            Self::All => node.as_tag().is_some(),
            Self::Attribute(attribute) => node
                .as_tag()
                .is_some_and(|t| t._attributes.get(*attribute).is_some()),
            Self::AttributeValue(attribute, value) => {
                check_attribute(node, attribute, value, |attr, value| attr == value)
            }
            Self::AttributeValueEndsWith(attribute, value) => {
                check_attribute(node, attribute, value, |attr, value| {
                    !value.is_empty() && attr.ends_with(value)
                })
            }
            Self::AttributeValueStartsWith(attribute, value) => {
                check_attribute(node, attribute, value, |attr, value| {
                    !value.is_empty() && attr.starts_with(value)
                })
            }
            Self::AttributeValueSubstring(attribute, value) => {
                check_attribute(node, attribute, value, |attr, value| {
                    !value.is_empty() && attr.contains(value)
                })
            }
            Self::AttributeValueWhitespacedContains(attribute, value) => {
                check_attribute(node, attribute, value, |attr, value| {
                    attr.split_whitespace().any(|x| x == value)
                })
            }
            #[cfg(feature = "std")]
            _ => false,
        }
    }
}

fn check_attribute<F>(node: &Node, attribute: &[u8], value: &[u8], callback: F) -> bool
where
    F: Fn(&str, &str) -> bool,
{
    let Ok(value) = core::str::from_utf8(value) else {
        return false;
    };
    node.as_tag().is_some_and(|t| {
        t._attributes
            .get(attribute)
            .flatten()
            .and_then(|attr| attr.try_as_utf8_str())
            .is_some_and(|attr| callback(attr, value))
    })
}

#[cfg(feature = "std")]
#[derive(Clone)]
pub(crate) struct Context {
    parent: Vec<Option<usize>>,
    previous: Vec<Option<usize>>,
    position: Vec<usize>,
}

#[cfg(feature = "std")]
impl Context {
    pub(crate) fn new(nodes: &[Node<'_>]) -> Self {
        let mut result = Self {
            parent: vec![None; nodes.len()],
            previous: vec![None; nodes.len()],
            position: vec![0; nodes.len()],
        };
        for (id, node) in nodes.iter().enumerate() {
            if let Some(tag) = node.as_tag() {
                let mut previous = None;
                let mut position = 0;
                for child in tag.children().top().iter() {
                    let child = child.get_inner() as usize;
                    if let Some(node) = nodes.get(child) {
                        result.parent[child] = Some(id);
                        if node.as_tag().is_some() {
                            position += 1;
                            result.previous[child] = previous;
                            result.position[child] = position;
                            previous = Some(child);
                        }
                    }
                }
            }
        }
        let mut previous = None;
        let mut position = 0;
        for (id, node) in nodes.iter().enumerate() {
            if result.parent[id].is_none() && node.as_tag().is_some() {
                position += 1;
                result.previous[id] = previous;
                result.position[id] = position;
                previous = Some(id);
            }
        }
        result
    }
}

#[cfg(feature = "std")]
impl<const N: usize> Selector<'_, N> {
    pub(crate) fn needs_context(&self) -> bool {
        match self {
            Self::And(a, b) | Self::Or(a, b) => a.needs_context() || b.needs_context(),
            Self::Not(a) => a.needs_context(),
            Self::Parent(..)
            | Self::Descendant(..)
            | Self::Adjacent(..)
            | Self::Sibling(..)
            | Self::NthChild(..)
            | Self::Has(..) => true,
            _ => false,
        }
    }

    pub(crate) fn matches_in(&self, id: usize, nodes: &[Node<'_>], ctx: &Context) -> bool {
        let Some(node) = nodes.get(id) else {
            return false;
        };
        if node.as_tag().is_none() {
            return false;
        }
        match self {
            Self::And(a, b) => a.matches_in(id, nodes, ctx) && b.matches_in(id, nodes, ctx),
            Self::Or(a, b) => a.matches_in(id, nodes, ctx) || b.matches_in(id, nodes, ctx),
            Self::Not(a) => !a.matches_in(id, nodes, ctx),
            Self::Parent(a, b) => {
                b.matches_in(id, nodes, ctx)
                    && ctx.parent[id].is_some_and(|p| a.matches_in(p, nodes, ctx))
            }
            Self::Adjacent(a, b) => {
                b.matches_in(id, nodes, ctx)
                    && ctx.previous[id].is_some_and(|p| a.matches_in(p, nodes, ctx))
            }
            Self::Descendant(a, b) | Self::Sibling(a, b) => {
                if !b.matches_in(id, nodes, ctx) {
                    return false;
                }
                let links = if matches!(self, Self::Descendant(..)) {
                    &ctx.parent
                } else {
                    &ctx.previous
                };
                let mut cursor = links[id];
                // Bounded traversal also tolerates a cyclic graph introduced by mutation.
                for _ in 0..nodes.len() {
                    let Some(p) = cursor else {
                        break;
                    };
                    if a.matches_in(p, nodes, ctx) {
                        return true;
                    }
                    cursor = links[p];
                }
                false
            }
            Self::NthChild(a, b) => {
                let delta = ctx.position[id] as i64 - i64::from(*b);
                let a = i64::from(*a);
                if a == 0 {
                    delta == 0
                } else {
                    delta % a == 0 && delta / a >= 0
                }
            }
            Self::Has(a) => {
                // Implicit descendant-relative arguments must not use the anchor
                // itself or its ancestors to satisfy their left-hand selector.
                let mut scoped = ctx.clone();
                for parent in &mut scoped.parent {
                    if *parent == Some(id) {
                        *parent = None;
                    }
                }
                nodes.iter().enumerate().any(|(candidate, _)| {
                    let mut cursor = ctx.parent[candidate];
                    for _ in 0..nodes.len() {
                        let Some(p) = cursor else {
                            break;
                        };
                        if p == id {
                            return a.matches_in(candidate, nodes, &scoped);
                        }
                        cursor = ctx.parent[p];
                    }
                    false
                })
            }
            _ => self.matches(node),
        }
    }
}
