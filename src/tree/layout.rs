use super::{Kind, TREE};
use serde::Serialize;
use std::{collections::HashMap, f64::consts::PI, sync::LazyLock};

pub static LAYOUT_JSON: LazyLock<String> =
    LazyLock::new(|| serde_json::to_string(&layout()).expect("layout must serialize"));

const GRID: f64 = 24.0;
const ROW_PITCH: f64 = 576.0;
const NODE_GAP: f64 = 72.0;
const DUMMY_WIDTH: f64 = 24.0;
const MARGIN: f64 = 240.0;
const TOOTH_RADIUS: f64 = 380.0 / 800.0;
const BODY_RADIUS: f64 = 320.0 / 800.0;
const TOOTH_ANGLE: f64 = 2.0 * PI / 14.0;
const ARROW_GAP: f64 = 4.0;
const ORDER_PASSES: usize = 32;
const PLACE_PASSES: usize = 200;
const BESIDE: [u32; 1] = [41];

#[derive(Serialize, Clone, Copy)]
struct Shape {
    cog: f64,
    label_width: f64,
    label_height: f64,
    label_offset: f64,
}

#[derive(Serialize)]
struct Shapes {
    item: Shape,
    token: Shape,
    origin: Shape,
}

const SHAPES: Shapes = Shapes {
    item: Shape {
        cog: 432.0,
        label_width: 456.0,
        label_height: 144.0,
        label_offset: 0.0,
    },
    token: Shape {
        cog: 288.0,
        label_width: 144.0,
        label_height: 64.0,
        label_offset: 112.0,
    },
    origin: Shape {
        cog: 320.0,
        label_width: 0.0,
        label_height: 0.0,
        label_offset: 0.0,
    },
};

fn shape(kind: Kind) -> Shape {
    match kind {
        Kind::Item => SHAPES.item,
        Kind::Token => SHAPES.token,
        Kind::Origin => SHAPES.origin,
    }
}

#[derive(Serialize)]
struct PlacedNode {
    id: u32,
    x: f64,
    y: f64,
}

#[derive(Serialize)]
struct PlacedEdge {
    from: u32,
    to: u32,
    path: String,
}

#[derive(Serialize)]
struct Layout {
    width: f64,
    height: f64,
    shapes: Shapes,
    nodes: Vec<PlacedNode>,
    edges: Vec<PlacedEdge>,
}

#[derive(Clone, Copy)]
struct Port {
    x: f64,
    y: f64,
    dx: f64,
    dy: f64,
}

impl Port {
    fn new(x: f64, y: f64, dx: f64, dy: f64) -> Self {
        let length = dx.hypot(dy);
        Port {
            x,
            y,
            dx: dx / length,
            dy: dy / length,
        }
    }

    fn arc(radius: f64, angle: f64, inward: bool) -> Self {
        let (sin, cos) = angle.sin_cos();
        let sign = if inward { -1.0 } else { 1.0 };
        Port::new(radius * cos, radius * sin, sign * cos, sign * sin)
    }
}

fn arc_angles(center: f64, spread: i32) -> impl Iterator<Item = f64> {
    (-spread..=spread)
        .rev()
        .map(move |k| center + k as f64 * TOOTH_ANGLE * center.signum())
}

fn tail_ports(kind: Kind) -> Vec<Port> {
    let s = shape(kind);
    let (half_width, half_height) = (s.label_width / 2.0, s.label_height / 2.0);
    let top_arc = arc_angles(-PI / 2.0, 2).map(move |a| Port::arc(s.cog * BODY_RADIUS, a, false));
    if s.label_width <= s.cog {
        return top_arc.collect();
    }
    match kind {
        Kind::Item => [Port::new(-half_width + 4.0, -half_height + 4.0, -0.6, -0.8)]
            .into_iter()
            .chain(top_arc)
            .chain([Port::new(half_width - 4.0, -half_height + 4.0, 0.6, -0.8)])
            .collect(),
        _ => top_arc.collect(),
    }
}

fn tip_ports(kind: Kind) -> Vec<Port> {
    let s = shape(kind);
    let (half_width, half_height) = (s.label_width / 2.0, s.label_height / 2.0);
    let bottom = s.label_offset + half_height + 3.0;
    let left = Port::new(-half_width - 3.0, bottom, 0.6, -0.8);
    let right = Port::new(half_width + 3.0, bottom, -0.6, -0.8);
    let arc = arc_angles(PI / 2.0, 2).map(|a| Port::arc(s.cog * TOOTH_RADIUS + ARROW_GAP, a, true));
    match kind {
        Kind::Origin => vec![],
        _ if s.label_width <= s.cog => arc.collect(),
        Kind::Item => [left].into_iter().chain(arc).chain([right]).collect(),
        Kind::Token => vec![
            left,
            Port::new(0.0, bottom + ARROW_GAP - 3.0, 0.0, -1.0),
            right,
        ],
    }
}

fn assign_ports(ports: &[Port], wants: &[f64]) -> Vec<usize> {
    let last = ports.len() as isize - 1;
    let mut picks: Vec<isize> = wants
        .iter()
        .map(|want| {
            (0..ports.len())
                .min_by(|&a, &b| {
                    (ports[a].x - want)
                        .abs()
                        .total_cmp(&(ports[b].x - want).abs())
                })
                .unwrap_or(0) as isize
        })
        .collect();
    if wants.len() <= ports.len() {
        for i in 1..picks.len() {
            picks[i] = picks[i].max(picks[i - 1] + 1).min(last);
        }
        for i in (0..picks.len().saturating_sub(1)).rev() {
            picks[i] = picks[i].min(picks[i + 1] - 1).max(0);
        }
    }
    picks.into_iter().map(|p| p as usize).collect()
}

struct Vertex {
    node: Option<usize>,
    rank: usize,
    width: f64,
    beside: Option<usize>,
    up: Vec<usize>,
    down: Vec<usize>,
}

fn ranks(preds: &[Vec<usize>], succs: &[Vec<usize>]) -> Vec<usize> {
    let n = preds.len();
    let mut indegree: Vec<usize> = preds.iter().map(Vec::len).collect();
    let mut order: Vec<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
    let mut head = 0;
    while head < order.len() {
        let u = order[head];
        head += 1;
        for &v in &succs[u] {
            indegree[v] -= 1;
            if indegree[v] == 0 {
                order.push(v);
            }
        }
    }
    assert_eq!(order.len(), n, "tree must be acyclic");

    let mut rank = vec![0; n];
    for &v in &order {
        rank[v] = preds[v].iter().map(|&u| rank[u] + 1).max().unwrap_or(0);
    }
    for _ in 0..8 {
        for &v in order.iter().rev() {
            if preds[v].is_empty() {
                continue;
            }
            let low = preds[v].iter().map(|&u| rank[u] + 1).max().unwrap();
            let high = succs[v].iter().map(|&w| rank[w] - 1).min().unwrap_or(low);
            rank[v] = if succs[v].len() > preds[v].len() {
                high
            } else {
                low
            };
        }
    }
    rank
}

fn crossings_between(vertices: &[Vertex], layer: &[usize], pos: &[usize]) -> usize {
    let segments: Vec<(usize, usize)> = layer
        .iter()
        .flat_map(|&a| vertices[a].down.iter().map(move |&b| (pos[a], pos[b])))
        .collect();
    let mut count = 0;
    for (i, &(a1, b1)) in segments.iter().enumerate() {
        for &(a2, b2) in &segments[i + 1..] {
            if (a1 < a2 && b1 > b2) || (a1 > a2 && b1 < b2) {
                count += 1;
            }
        }
    }
    count
}

fn total_crossings(vertices: &[Vertex], layers: &[Vec<usize>], pos: &[usize]) -> usize {
    layers
        .iter()
        .map(|layer| crossings_between(vertices, layer, pos))
        .sum()
}

fn reindex(layers: &[Vec<usize>], pos: &mut [usize]) {
    for layer in layers {
        for (i, &v) in layer.iter().enumerate() {
            pos[v] = i;
        }
    }
}

fn pair_crossings(vertices: &[Vertex], pos: &[usize], left: usize, right: usize) -> usize {
    let count = |a: &[usize], b: &[usize]| {
        a.iter()
            .map(|&x| b.iter().filter(|&&y| pos[x] > pos[y]).count())
            .sum::<usize>()
    };
    count(&vertices[left].up, &vertices[right].up)
        + count(&vertices[left].down, &vertices[right].down)
}

fn transpose(vertices: &[Vertex], layers: &mut [Vec<usize>], pos: &mut [usize]) {
    let mut improved = true;
    while improved {
        improved = false;
        for layer in layers.iter_mut() {
            for i in 1..layer.len() {
                let (left, right) = (layer[i - 1], layer[i]);
                if pair_crossings(vertices, pos, right, left)
                    < pair_crossings(vertices, pos, left, right)
                {
                    layer.swap(i - 1, i);
                    pos[left] = i;
                    pos[right] = i - 1;
                    improved = true;
                }
            }
        }
    }
}

fn barycenter_sweep(
    vertices: &[Vertex],
    layers: &mut [Vec<usize>],
    pos: &mut [usize],
    downward: bool,
) {
    let order: Vec<usize> = if downward {
        (1..layers.len()).collect()
    } else {
        (0..layers.len().saturating_sub(1)).rev().collect()
    };
    for r in order {
        let keys: HashMap<usize, f64> = layers[r]
            .iter()
            .map(|&v| {
                let neighbours = if downward {
                    &vertices[v].up
                } else {
                    &vertices[v].down
                };
                let key = if neighbours.is_empty() {
                    pos[v] as f64
                } else {
                    neighbours.iter().map(|&u| pos[u] as f64).sum::<f64>() / neighbours.len() as f64
                };
                (v, key)
            })
            .collect();
        layers[r].sort_by(|a, b| keys[a].total_cmp(&keys[b]));
        reindex(&layers[r..=r], pos);
    }
}

fn order_layers(vertices: &[Vertex], layers: &mut Vec<Vec<usize>>) {
    let mut pos = vec![0; vertices.len()];
    reindex(layers, &mut pos);
    let mut best = layers.clone();
    let mut best_count = total_crossings(vertices, layers, &pos);
    for pass in 0..ORDER_PASSES {
        barycenter_sweep(vertices, layers, &mut pos, pass % 2 == 0);
        transpose(vertices, layers, &mut pos);
        let count = total_crossings(vertices, layers, &pos);
        if count < best_count {
            best_count = count;
            best = layers.clone();
        }
    }
    *layers = best;
}

fn group_categories(vertices: &[Vertex], layers: &mut [Vec<usize>]) {
    let category = |v: usize| {
        vertices[v]
            .node
            .and_then(|i| TREE.nodes[i].category.as_deref())
    };
    let mut pos = vec![0; vertices.len()];
    reindex(layers, &mut pos);
    for r in 0..layers.len() {
        for &lead in &layers[r].clone() {
            let Some(group) = category(lead) else {
                continue;
            };
            let members: Vec<usize> = layers[r]
                .iter()
                .copied()
                .filter(|&v| category(v) == Some(group) && vertices[v].up == vertices[lead].up)
                .collect();
            let start = layers[r].iter().position(|v| members.contains(v)).unwrap();
            let mut candidate: Vec<usize> = layers[r][..start]
                .iter()
                .copied()
                .filter(|v| !members.contains(v))
                .collect();
            candidate.extend(&members);
            candidate.extend(layers[r][start..].iter().filter(|v| !members.contains(v)));
            let mut trial = pos.clone();
            reindex(std::slice::from_ref(&candidate), &mut trial);
            if total_crossings(vertices, layers, &trial) <= total_crossings(vertices, layers, &pos)
            {
                layers[r] = candidate;
                pos = trial;
            }
        }
    }
}

fn separation(vertices: &[Vertex], a: usize, b: usize) -> f64 {
    let gap = match (vertices[a].node, vertices[b].node) {
        (Some(_), Some(_)) => NODE_GAP,
        (None, None) => DUMMY_WIDTH,
        _ => NODE_GAP / 2.0,
    };
    (vertices[a].width + vertices[b].width) / 2.0 + gap
}

fn place_layer(vertices: &[Vertex], layer: &[usize], xs: &mut [f64]) {
    let mut offsets = vec![0.0; layer.len()];
    for i in 1..layer.len() {
        offsets[i] = offsets[i - 1] + separation(vertices, layer[i - 1], layer[i]);
    }
    let mut blocks: Vec<(f64, f64, usize)> = vec![];
    for (i, &v) in layer.iter().enumerate() {
        let neighbours: Vec<usize> = vertices[v]
            .up
            .iter()
            .chain(&vertices[v].down)
            .copied()
            .collect();
        let desired = if let Some(p) = vertices[v].beside {
            xs[p] + separation(vertices, p, v)
        } else if neighbours.is_empty() {
            xs[v]
        } else {
            neighbours.iter().map(|&u| xs[u]).sum::<f64>() / neighbours.len() as f64
        };
        let weight = if vertices[v].node.is_some() { 1.0 } else { 2.0 };
        blocks.push((weight * (desired - offsets[i]), weight, 1));
        while blocks.len() > 1 {
            let (sum, weight, count) = blocks[blocks.len() - 1];
            let (prev_sum, prev_weight, prev_count) = blocks[blocks.len() - 2];
            if prev_sum / prev_weight <= sum / weight {
                break;
            }
            blocks.pop();
            *blocks.last_mut().unwrap() =
                (prev_sum + sum, prev_weight + weight, prev_count + count);
        }
    }
    let mut i = 0;
    for (sum, weight, count) in blocks {
        for _ in 0..count {
            xs[layer[i]] = sum / weight + offsets[i];
            i += 1;
        }
    }
}

fn snap(value: f64) -> f64 {
    ((value - GRID / 2.0) / GRID).round() * GRID + GRID / 2.0
}

fn snap_up(value: f64) -> f64 {
    ((value - GRID / 2.0) / GRID).ceil() * GRID + GRID / 2.0
}

fn place(vertices: &[Vertex], layers: &[Vec<usize>]) -> Vec<f64> {
    let mut xs = vec![0.0; vertices.len()];
    for layer in layers {
        for i in 1..layer.len() {
            xs[layer[i]] = xs[layer[i - 1]] + separation(vertices, layer[i - 1], layer[i]);
        }
    }
    for pass in 0..PLACE_PASSES {
        if pass % 2 == 0 {
            layers
                .iter()
                .for_each(|layer| place_layer(vertices, layer, &mut xs));
        } else {
            layers
                .iter()
                .rev()
                .for_each(|layer| place_layer(vertices, layer, &mut xs));
        }
    }
    for layer in layers {
        for i in 0..layer.len() {
            let snapped = snap(xs[layer[i]]);
            xs[layer[i]] = match i {
                0 => snapped,
                _ => snapped.max(snap_up(
                    xs[layer[i - 1]] + separation(vertices, layer[i - 1], layer[i]),
                )),
            };
        }
    }
    let left = (0..vertices.len())
        .map(|v| xs[v] - vertices[v].width / 2.0)
        .fold(f64::INFINITY, f64::min);
    let shift = ((MARGIN - left) / GRID).round() * GRID;
    xs.iter_mut().for_each(|x| *x += shift);
    xs
}

fn bezier(points: &[(f64, f64, f64, f64)]) -> String {
    let mut path = format!("M{:.1} {:.1}", points[0].0, points[0].1);
    for pair in points.windows(2) {
        let (ax, ay, adx, ady) = pair[0];
        let (bx, by, bdx, bdy) = pair[1];
        let reach = 0.45 * (bx - ax).hypot(by - ay);
        path += &format!(
            " C{:.1} {:.1} {:.1} {:.1} {:.1} {:.1}",
            ax + adx * reach,
            ay + ady * reach,
            bx - bdx * reach,
            by - bdy * reach,
            bx,
            by
        );
    }
    path
}

fn layout() -> Layout {
    let nodes = &TREE.nodes;
    let index: HashMap<u32, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id, i))
        .collect();
    let edges: Vec<(usize, usize)> = TREE
        .edges
        .iter()
        .map(|e| (index[&e.from], index[&e.to]))
        .collect();
    let mut preds = vec![vec![]; nodes.len()];
    let mut succs = vec![vec![]; nodes.len()];
    for &(u, v) in &edges {
        succs[u].push(v);
        preds[v].push(u);
    }
    let mut rank = ranks(&preds, &succs);
    let beside: Vec<Option<usize>> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| BESIDE.contains(&node.id).then(|| preds[i][0]))
        .collect();
    for (i, p) in beside.iter().enumerate() {
        if let Some(p) = *p {
            rank[i] = rank[p];
        }
    }

    let mut vertices: Vec<Vertex> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let s = shape(node.kind);
            Vertex {
                node: Some(i),
                rank: rank[i],
                width: s.cog.max(s.label_width),
                beside: beside[i],
                up: vec![],
                down: vec![],
            }
        })
        .collect();
    let chains: Vec<Vec<usize>> = edges
        .iter()
        .map(|&(u, v)| {
            let mut chain = vec![u];
            for r in rank[u] + 1..rank[v] {
                vertices.push(Vertex {
                    node: None,
                    rank: r,
                    width: DUMMY_WIDTH,
                    beside: None,
                    up: vec![],
                    down: vec![],
                });
                chain.push(vertices.len() - 1);
            }
            chain.push(v);
            for link in chain.windows(2).filter(|_| beside[v].is_none()) {
                vertices[link[0]].down.push(link[1]);
                vertices[link[1]].up.push(link[0]);
            }
            chain
        })
        .collect();

    let mut layers = vec![vec![]; rank.iter().max().map_or(0, |r| r + 1)];
    let mut seen = vec![false; vertices.len()];
    let mut stack: Vec<usize> = (0..nodes.len())
        .filter(|&i| preds[i].is_empty())
        .rev()
        .collect();
    while let Some(v) = stack.pop() {
        if std::mem::replace(&mut seen[v], true) {
            continue;
        }
        layers[vertices[v].rank].push(v);
        stack.extend(vertices[v].down.iter().rev().filter(|&&w| !seen[w]));
    }
    order_layers(&vertices, &mut layers);
    group_categories(&vertices, &mut layers);
    for (i, p) in beside.iter().enumerate() {
        if let Some(p) = *p {
            let layer = &mut layers[rank[p]];
            let at = layer.iter().position(|&v| v == p).unwrap();
            layer.insert(at + 1, i);
        }
    }
    let xs = place(&vertices, &layers);
    let top_rank = layers.len() - 1;
    let y_of = |r: usize| MARGIN + GRID / 2.0 + (top_rank - r) as f64 * ROW_PITCH;

    let mut tails = vec![None; chains.len()];
    let mut tips = vec![None; chains.len()];
    for (i, node) in nodes.iter().enumerate() {
        let mut outgoing: Vec<(usize, f64)> = chains
            .iter()
            .enumerate()
            .filter(|(_, chain)| chain[0] == i && beside[chain[chain.len() - 1]].is_none())
            .map(|(e, chain)| (e, (xs[chain[1]] - xs[i]) * 0.4))
            .collect();
        outgoing.sort_by(|a, b| a.1.total_cmp(&b.1));
        let ports = tail_ports(node.kind);
        let wants: Vec<f64> = outgoing.iter().map(|o| o.1).collect();
        for ((e, _), pick) in outgoing.iter().zip(assign_ports(&ports, &wants)) {
            tails[*e] = Some(ports[pick]);
        }

        let mut incoming: Vec<(usize, f64)> = chains
            .iter()
            .enumerate()
            .filter(|(_, chain)| chain[chain.len() - 1] == i && beside[i].is_none())
            .map(|(e, chain)| (e, (xs[chain[chain.len() - 2]] - xs[i]) * 0.4))
            .collect();
        incoming.sort_by(|a, b| a.1.total_cmp(&b.1));
        let ports = tip_ports(node.kind);
        let wants: Vec<f64> = incoming.iter().map(|o| o.1).collect();
        for ((e, _), pick) in incoming.iter().zip(assign_ports(&ports, &wants)) {
            tips[*e] = Some(ports[pick]);
        }
    }

    let placed_edges = chains
        .iter()
        .enumerate()
        .map(|(e, chain)| {
            let (u, v) = (chain[0], chain[chain.len() - 1]);
            let (tail, tip) = match beside[v] {
                Some(_) => {
                    let (from, to) = (shape(nodes[u].kind), shape(nodes[v].kind));
                    (
                        Port::new(
                            (from.label_width / 2.0).max(from.cog * BODY_RADIUS),
                            0.0,
                            1.0,
                            0.0,
                        ),
                        Port::arc(to.cog * TOOTH_RADIUS + ARROW_GAP, PI, true),
                    )
                }
                None => (tails[e].unwrap(), tips[e].unwrap()),
            };
            let mut points = vec![(xs[u] + tail.x, y_of(rank[u]) + tail.y, tail.dx, tail.dy)];
            points.extend(
                chain[1..chain.len() - 1]
                    .iter()
                    .map(|&d| (xs[d], y_of(vertices[d].rank), 0.0, -1.0)),
            );
            points.push((xs[v] + tip.x, y_of(rank[v]) + tip.y, tip.dx, tip.dy));
            PlacedEdge {
                from: nodes[u].id,
                to: nodes[v].id,
                path: bezier(&points),
            }
        })
        .collect();

    let right = (0..vertices.len())
        .map(|v| xs[v] + vertices[v].width / 2.0)
        .fold(0.0, f64::max);
    Layout {
        width: snap_up(right + MARGIN) - GRID / 2.0,
        height: snap_up(y_of(0) + SHAPES.item.cog / 2.0 + MARGIN) - GRID / 2.0,
        shapes: SHAPES,
        nodes: nodes
            .iter()
            .enumerate()
            .map(|(i, node)| PlacedNode {
                id: node.id,
                x: xs[i],
                y: y_of(rank[i]),
            })
            .collect(),
        edges: placed_edges,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nodes_in_a_row_do_not_overlap() {
        let layout = layout();
        assert_eq!(layout.nodes.len(), TREE.nodes.len());
        assert_eq!(layout.edges.len(), TREE.edges.len());
        for a in &layout.nodes {
            for b in &layout.nodes {
                let width = |id| {
                    let s = shape(TREE.node(id).unwrap().kind);
                    s.cog.max(s.label_width)
                };
                if a.id != b.id && a.y == b.y {
                    assert!((a.x - b.x).abs() >= (width(a.id) + width(b.id)) / 2.0 + NODE_GAP);
                }
            }
        }
    }
}
