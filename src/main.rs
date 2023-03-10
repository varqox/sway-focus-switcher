use std::process::{Command, Stdio};

use clap::{Parser, ValueEnum};

mod swaymsg {
    pub(crate) mod tree {
        use crate::swaymsg::tree::node::Node;

        pub(crate) mod node {
            use serde::Deserialize;

            #[derive(Debug, Deserialize)]
            #[serde(tag = "type")]
            #[serde(rename_all = "lowercase")]
            pub(crate) enum Node {
                Root {
                    nodes: Vec<Node>,
                },
                Output {
                    nodes: Vec<Node>,
                },
                Workspace {
                    nodes: Vec<Node>,
                },
                Con {
                    id: u32,
                    focused: bool,
                    nodes: Vec<Node>,
                },
            }
        }

        struct WTFSearching<'a> {
            is_rightmost_window_focused: bool,
            leftmost_window: &'a node::Node,
        }

        enum WindowToFocus<'a> {
            Found(&'a node::Node),
            Searching(WTFSearching<'a>),
        }

        fn impl_next_window_to_focus<'a>(
            node: &'a node::Node,
            reversed_order: bool,
        ) -> WindowToFocus {
            use node::Node::*;
            let reduce = |nodes: &'a Vec<Node>, focused| -> WindowToFocus {
                use WindowToFocus::*;
                if nodes.is_empty() {
                    return Searching(WTFSearching {
                        is_rightmost_window_focused: focused,
                        leftmost_window: node,
                    });
                }
                let iter = |iter: &mut dyn Iterator<Item = &'a node::Node>| {
                    iter.map(|node| impl_next_window_to_focus(node, reversed_order))
                        .reduce(|res, e| match res {
                            Found(x) => Found(x),
                            Searching(a) => match e {
                                Found(x) => Found(x),
                                Searching(b) => {
                                    if a.is_rightmost_window_focused {
                                        Found(b.leftmost_window)
                                    } else {
                                        Searching(WTFSearching {
                                            is_rightmost_window_focused: b
                                                .is_rightmost_window_focused,
                                            leftmost_window: a.leftmost_window,
                                        })
                                    }
                                }
                            },
                        })
                        .unwrap()
                };
                if reversed_order {
                    iter(&mut nodes.iter().rev())
                } else {
                    iter(&mut nodes.iter())
                }
            };

            match node {
                Root { .. } | Output { .. } => panic!(),
                Workspace { nodes } => reduce(nodes, false),
                Con { focused, nodes, .. } => reduce(nodes, *focused),
            }
        }

        pub(crate) fn next_window_to_focus(
            node: &node::Node,
            reversed_order: bool,
        ) -> Option<&node::Node> {
            use node::Node::*;
            match node {
                Root { nodes } | Output { nodes } => nodes
                    .iter()
                    .find_map(|node| next_window_to_focus(node, reversed_order)),
                Workspace { .. } => match impl_next_window_to_focus(node, reversed_order) {
                    WindowToFocus::Found(res) => Some(res),
                    WindowToFocus::Searching(WTFSearching {
                        is_rightmost_window_focused,
                        leftmost_window,
                    }) => {
                        if is_rightmost_window_focused {
                            Some(leftmost_window)
                        } else {
                            None
                        }
                    }
                },
                Con { .. } => panic!(),
            }
        }
    }
}

/// Switches focus between windows on the current workspace
#[derive(Parser)]
struct Cli {
    // Direction
    #[arg(value_enum)]
    direction: Direction,
}

#[derive(Clone, ValueEnum)]
enum Direction {
    Next,
    Prev,
}

fn main() {
    let cli = Cli::parse();

    let swamsg_tree: swaymsg::tree::node::Node = serde_json::from_slice(
        &Command::new("swaymsg")
            .args(["-t", "get_tree"])
            .stderr(Stdio::null()) // ignore stderr for performance reasons
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();

    if let Some(swaymsg::tree::node::Node::Con { id, .. }) = swaymsg::tree::next_window_to_focus(
        &swamsg_tree,
        match cli.direction {
            Direction::Next => false,
            Direction::Prev => true,
        },
    ) {
        Command::new("swaymsg")
            .args([&format!("[con_id={}]", id), "focus"])
            .status()
            .unwrap();
    }
}
