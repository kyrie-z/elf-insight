use crate::app::{App, Focus};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

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

    fn collect_flat(&self) -> Vec<(&TreeNode, u8, Vec<usize>)> {
        let mut result = Vec::new();
        Self::collect(&self.nodes, 0, &mut vec![], &mut result);
        result
    }

    fn collect<'a>(nodes: &'a [TreeNode], depth: u8, path: &Vec<usize>, result: &mut Vec<(&'a TreeNode, u8, Vec<usize>)>) {
        for (i, node) in nodes.iter().enumerate() {
            let mut node_path = path.clone();
            node_path.push(i);
            result.push((node, depth, node_path.clone()));
            if node.expanded {
                Self::collect(&node.children, depth + 1, &node_path, result);
            }
        }
    }

    pub fn node_at_index(&self, idx: usize) -> Option<TreeNodeType> {
        let flat = self.collect_flat();
        flat.get(idx).map(|(n, _, _)| n.node_type.clone())
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
        let flat = self.collect_flat();
        if idx + 1 < flat.len() {
            self.list_state.select(Some(idx + 1));
            self.selected_node = self.node_at_index(idx + 1);
        }
    }

    pub fn toggle_expand(&mut self) {
        let flat = self.collect_flat();
        let idx = self.list_state.selected().unwrap_or(0);
        if let Some((node, _, _)) = flat.get(idx) {
            if !node.children.is_empty() {
                let node_type = node.node_type.clone();
                let node_label = node.label.clone();
                // Search top-level nodes
                for top_node in &mut self.nodes {
                    if top_node.node_type == node_type && top_node.label == node_label {
                        top_node.expanded = !top_node.expanded;
                        self.rebuild_flat_list();
                        self.selected_node = self.node_at_index(idx);
                        return;
                    }
                    // Search children
                    for child in &mut top_node.children {
                        if child.node_type == node_type && child.label == node_label {
                            child.expanded = !child.expanded;
                            self.rebuild_flat_list();
                            self.selected_node = self.node_at_index(idx);
                            return;
                        }
                    }
                }
            }
        }
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let flat = app.tree.collect_flat();

    let items: Vec<ListItem> = flat
        .iter()
        .map(|(node, depth, _path)| {
            let indent = "  ".repeat(*depth as usize);
            let prefix = if !node.children.is_empty() {
                if node.expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };
            ListItem::new(format!("{}{}{}", indent, prefix, node.label))
        })
        .collect();

    let border_style = if app.focus == Focus::Tree {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Navigation").border_style(border_style))
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White));

    f.render_stateful_widget(list, area, &mut app.tree.list_state);
}