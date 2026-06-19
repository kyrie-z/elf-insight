use ratatui::widgets::ListState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeNodeType {
    Overview,
    ElfHeader,
    SectionsGroup,
    SectionHeader { index: usize },
    SectionBody { index: usize },
    SegmentsGroup,
    Segment { index: usize },
    SymbolsGroup,
    Symbol { index: usize },
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub label: String,
    pub node_type: TreeNodeType,
    pub depth: u8,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
}

#[derive(Debug, Clone)]
pub struct TreeState {
    pub nodes: Vec<TreeNode>,
    pub flat_list: Vec<usize>,
    pub list_state: ListState,
    pub selected_node: Option<TreeNodeType>,
}

impl TreeState {
    pub fn new(nodes: Vec<TreeNode>) -> Self {
        let mut state = TreeState {
            nodes,
            flat_list: Vec::new(),
            list_state: ListState::default(),
            selected_node: None,
        };
        state.rebuild_flat_list();
        state.list_state.select(Some(0));
        state.selected_node = state.node_at_index(0);
        state
    }

    pub fn rebuild_flat_list(&mut self) {
        self.flat_list.clear();
        Self::flatten(&self.nodes, &mut self.flat_list);
    }

    fn flatten(nodes: &[TreeNode], result: &mut Vec<usize>) {
        for (i, node) in nodes.iter().enumerate() {
            result.push(i);
            if node.expanded {
                Self::flatten(&node.children, result);
            }
        }
    }

    pub fn node_at_index(&self, idx: usize) -> Option<TreeNodeType> {
        let mut count = 0;
        Self::find_node(&self.nodes, idx, &mut count)
    }

    fn find_node(nodes: &[TreeNode], target: usize, count: &mut usize) -> Option<TreeNodeType> {
        for node in nodes {
            if *count == target {
                return Some(node.node_type.clone());
            }
            *count += 1;
            if node.expanded {
                if let Some(t) = Self::find_node(&node.children, target, count) {
                    return Some(t);
                }
            }
        }
        None
    }

    pub fn move_up(&mut self) {
        let idx = self.list_state.selected().unwrap_or(0);
        if idx > 0 {
            self.list_state.select(Some(idx - 1));
            self.selected_node = self.node_at_index(idx - 1);
        }
    }

    pub fn move_down(&mut self) {
        let idx = self.list_state.selected().unwrap_or(0);
        if idx + 1 < self.flat_list.len() {
            self.list_state.select(Some(idx + 1));
            self.selected_node = self.node_at_index(idx + 1);
        }
    }

    pub fn toggle_expand(&mut self) {
        let idx = self.list_state.selected().unwrap_or(0);
        if let Some(node) = self.node_at_flat_index(idx) {
            if !node.children.is_empty() {
                node.expanded = !node.expanded;
                self.rebuild_flat_list();
                self.list_state.select(Some(idx));
                self.selected_node = self.node_at_index(idx);
            }
        }
    }

    fn node_at_flat_index(&mut self, target: usize) -> Option<&mut TreeNode> {
        let mut count = 0;
        Self::find_node_mut(&mut self.nodes, target, &mut count)
    }

    fn find_node_mut<'a>(
        nodes: &'a mut [TreeNode],
        target: usize,
        count: &mut usize,
    ) -> Option<&'a mut TreeNode> {
        for node in nodes.iter_mut() {
            if *count == target {
                return Some(node);
            }
            *count += 1;
            if node.expanded {
                if let Some(n) = Self::find_node_mut(&mut node.children, target, count) {
                    return Some(n);
                }
            }
        }
        None
    }
}

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .tree
        .flat_list
        .iter()
        .map(|_| {
            let (node, depth) = get_flat_node(&app.tree.nodes, app.tree.flat_list.as_slice());
            let indent = "  ".repeat(depth as usize);
            ListItem::new(format!("{}{}", indent, node.label))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Navigation"))
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(list, area, &mut app.tree.list_state);
}

fn get_flat_node<'a>(nodes: &'a [TreeNode], _flat_indices: &[usize]) -> (&'a TreeNode, u8) {
    (&nodes[0], 0)
}