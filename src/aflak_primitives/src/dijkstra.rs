/// This code is based on the Example of Module std::collections::binary_heap
/// https://doc.rust-lang.org/std/collections/binary_heap/index.html
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Total<T>(pub T);

impl<T: PartialEq> Eq for Total<T> {}

impl<T: PartialOrd> Ord for Total<T> {
    fn cmp(&self, other: &Total<T>) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct State {
    cost: Total<f32>,
    position: usize,
}

// The priority queue depends on `Ord`.
// Explicitly implement the trait so the queue becomes a min-heap
// instead of a max-heap.
impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // Notice that the we flip the ordering on costs.
        // In case of a tie we compare positions - this step is necessary
        // to make implementations of `PartialEq` and `Ord` consistent.
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.position.cmp(&other.position))
    }
}

// `PartialOrd` needs to be implemented as well.
impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Each node is represented as a `usize`, for a shorter implementation.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct Edge {
    pub node: usize,
    pub cost: Total<f32>,
}

// Dijkstra's shortest path algorithm.

// Start at `start` and use `dist` to track the current shortest distance
// to each node. This implementation isn't memory-efficient as it may leave duplicate
// nodes in the queue. It also uses `usize::MAX` as a sentinel value,
// for a simpler implementation.
pub(crate) fn shortest_path(
    adj_list: &Vec<Vec<Edge>>,
    start: usize,
    goal: usize,
) -> Option<Total<f32>>
where
{
    // dist[node] = current shortest distance from `start` to `node`
    let mut dist: Vec<_> = (0..adj_list.len()).map(|_| Total(f32::MAX)).collect();

    let mut heap = BinaryHeap::new();

    // We're at `start`, with a zero cost
    dist[start] = Total(0.0);
    heap.push(State {
        cost: Total(0.0),
        position: start,
    });

    // Examine the frontier with lower cost nodes first (min-heap)
    while let Some(State { cost, position }) = heap.pop() {
        // Alternatively we could have continued to find all shortest paths
        if position == goal {
            return Some(cost);
        }

        // Important as we may have already found a better way
        if cost > dist[position] {
            continue;
        }

        // For each node we can reach, see if we can find a way with
        // a lower cost going through this node
        for edge in &adj_list[position] {
            let Total(nc) = cost;
            let Total(ec) = edge.cost;
            let next = State {
                cost: Total(nc + ec),
                position: edge.node,
            };

            // If so, add it to the frontier and continue
            if next.cost < dist[next.position] {
                heap.push(next);
                // Relaxation, we have now found a better way
                dist[next.position] = next.cost;
            }
        }
    }

    // Goal not reachable
    None
}

#[test]
fn test_dijkstra() {
    // This is the directed graph we're going to use.
    // The node numbers correspond to the different states,
    // and the edge weights symbolize the cost of moving
    // from one node to another.
    // Note that the edges are one-way.
    //
    //                  7
    //          +-----------------+
    //          |                 |
    //          v   1        2    |  2
    //          0 -----> 1 -----> 3 ---> 4
    //          |        ^        ^      ^
    //          |        | 1      |      |
    //          |        |        | 3    | 1
    //          +------> 2 -------+      |
    //           10      |               |
    //                   +---------------+
    //
    // The graph is represented as an adjacency list where each index,
    // corresponding to a node value, has a list of outgoing edges.
    // Chosen for its efficiency.
    let graph = vec![
        // Node 0
        vec![Edge { node: 2, cost: 10 }, Edge { node: 1, cost: 1 }],
        // Node 1
        vec![Edge { node: 3, cost: 2 }],
        // Node 2
        vec![
            Edge { node: 1, cost: 1 },
            Edge { node: 3, cost: 3 },
            Edge { node: 4, cost: 1 },
        ],
        // Node 3
        vec![Edge { node: 0, cost: 7 }, Edge { node: 4, cost: 2 }],
        // Node 4
        vec![],
    ];

    assert_eq!(shortest_path(&graph, 0, 1), Some(1));
    assert_eq!(shortest_path(&graph, 0, 3), Some(3));
    assert_eq!(shortest_path(&graph, 3, 0), Some(7));
    assert_eq!(shortest_path(&graph, 0, 4), Some(5));
    assert_eq!(shortest_path(&graph, 4, 0), None);
}
