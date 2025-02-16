use core::borrow::Borrow;

use crate::physics::{Affect, Combine, Position};

use itertools::Itertools;
use ultraviolet::{
    int::{IVec2, UVec2},
    Vec2,
};

pub fn aabb<I: IntoIterator<Item = Vec2>>(points: I) -> (Vec2, Vec2) {
    let (min, max) = points.into_iter().fold(
        (Vec2::broadcast(f32::MAX), Vec2::broadcast(f32::MIN)),
        |(min, max), point| (min.min_by_component(point), max.max_by_component(point)),
    );

    (min, max - min)
}

pub struct Quadtree {
    inner: Vec<Option<UVec2>>,
    points: Vec<Vec<usize>>,
    quads: Vec<usize>,

    pub pos: Vec2,
    pub size: f32,
}

fn cast_to_ivec2(v: UVec2) -> IVec2 {
    IVec2::new(v.x as i32, v.y as i32)
}

impl Quadtree {
    pub fn new(pos: Vec2, size: f32) -> Self {
        Self {
            inner: Default::default(),
            points: Default::default(),
            quads: Default::default(),
            pos,
            size,
        }
    }

    pub fn children<'a>(&'a self, parent: usize) -> impl Iterator<Item = usize> + use<'a> {
        (parent * 4 + 1..parent * 4 + 5).filter(|&i| self.inner[i].is_some())
    }

    pub fn set_size(&mut self, size: f32) -> &mut Self {
        self.size = size;
        self
    }

    pub fn set_pos(&mut self, pos: Vec2) -> &mut Self {
        self.pos = pos;
        self
    }

    pub fn len_for_depth(depth: usize) -> usize {
        ((4 << (2 * depth)) - 1) / 3
    }

    pub fn grow(&mut self, depth: usize) -> &mut Self {
        let tree_len = Self::len_for_depth(depth);
        let points_len = 1 << (2 * depth);

        if self.inner.len() < tree_len {
            self.inner.resize(tree_len, None);
        }

        if self.points.len() < points_len {
            self.points.resize_with(points_len, Default::default);
        }

        self
    }

    pub fn clear(&mut self) -> &mut Self {
        for quad in self.quads.drain(0..self.quads.len()) {
            self.inner[quad] = None;
        }

        self.points.iter_mut().for_each(Vec::clear);
        self
    }

    pub fn collate(&mut self) -> &mut Self {
        for i in (1..self.inner.len()).rev() {
            if let Some(quad) = self.inner[i] {
                self.inner[(i - 1) / 4].get_or_insert(quad / 2);
                self.quads.push(i);
            }
        }

        self.quads.push(0);
        self.quads.reverse();
        self
    }

    pub fn build_from_objects<T, U>(
        &mut self,
        objects: &[T],
        points: impl IntoIterator<Item = usize>,
    ) where
        T: Borrow<U>,
        U: Position + Affect<Subject = U> + Combine + Default,
    {
        let depth = ceil_log4(objects.len() * 2);
        self.grow(depth as usize);

        // Index at which leaf quads start
        let leaf_idx = self.inner.len() - self.points.len();

        // Scale position vector by this before flooring
        let scale = (1 << depth) as f32 / self.size;

        for (id, pos) in points
            .into_iter()
            .map(|id| objects[id].borrow().pos() - self.pos)
            .enumerate()
            .filter(|(_, pos)| pos.component_min() >= 0. && pos.component_max() < self.size)
        {
            let coords = UVec2::try_from(pos * scale).unwrap();
            let z = zorder::index_of(coords.as_array()) as usize;

            self.points[z].push(id);
            self.inner[leaf_idx + z].get_or_insert(coords);
        }

        self.collate();
    }
}

#[derive(Default)]
struct Quad<T: Default> {
    sum: T,
    acc: Vec2,
    near: Vec<usize>,
}

impl<T: Default> Quad<T> {
    fn reset(&mut self) {
        self.sum = T::default();
        self.acc = Vec2::zero();
        self.near.clear();
    }
}

pub struct Reactions<T: Default> {
    inner: Vec<Quad<T>>,
}

impl<T: Position + Affect<Subject = T> + Combine + Default> Reactions<T> {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub fn clear(&mut self) -> &mut Self {
        self.inner.iter_mut().for_each(Quad::reset);
        self
    }

    fn grow(&mut self, len: usize) -> &mut Self {
        if self.inner.len() < len {
            self.inner.resize_with(len, Default::default);
        }

        self
    }

    pub fn compute_on<U>(&mut self, objects: &[U], tree: &Quadtree, field: &mut [Vec2])
    where
        U: Borrow<T>,
    {
        compute(objects, tree, self, field);
    }
}

fn compute<U, T>(objects: &[U], tree: &Quadtree, reactions: &mut Reactions<T>, field: &mut [Vec2])
where
    U: Borrow<T>,
    T: Position + Affect<Subject = T> + Combine + Default,
{
    reactions.grow(tree.inner.len());

    let leaf_idx = tree.inner.len() - tree.points.len();

    // We could use slice pattern matching but it's less annoying to just use pointers
    let quads = reactions.inner.as_mut_ptr();

    unsafe {
        core::hint::assert_unchecked(tree.quads.is_sorted_by(usize::lt));
    }

    // Compute sums
    for &quad in tree.quads.iter().rev() {
        if quad >= leaf_idx {
            for &point in &tree.points[quad - leaf_idx] {
                reactions.inner[quad].sum.combine(objects[point].borrow());
            }
        }

        unsafe {
            (*quads.add((quad - 1) / 4))
                .sum
                .combine(&(*quads.add(quad)).sum);
        }
    }

    reactions.inner[0].near.push(0);

    // Partition and compute near/far field interactions
    for &quad in tree.quads[1..].iter() {
        let coords = cast_to_ivec2(tree.inner[quad].unwrap());

        for candidate in unsafe { &*quads.add((quad - 1) / 4) }
            .near
            .iter()
            .flat_map(|&q| tree.children(q))
        {
            let near = (cast_to_ivec2(tree.inner[candidate].unwrap()) - coords)
                .abs()
                .component_max()
                <= 1;

            if near {
                reactions.inner[quad].near.push(candidate);
            } else {
                let effect = reactions.inner[candidate]
                    .sum
                    .effect_on(&reactions.inner[quad].sum);
                reactions.inner[quad].acc += effect;
            }
        }
    }

    for &quad in tree.quads[1..].iter() {
        // Propagate parent acceleration
        reactions.inner[quad].acc += unsafe { (*quads.add((quad - 1) / 4)).acc };

        if quad < leaf_idx {
            continue;
        }

        let target = &tree.points[quad - leaf_idx];

        // Compute pairwise interactions between contained points and points of neighbouring cells
        for &neighbour in reactions.inner[quad].near.iter().filter(|&&q| q != quad) {
            for &point in target {
                if tree.points[neighbour - leaf_idx].len() > 24 {
                    field[point] += reactions.inner[neighbour]
                        .sum
                        .effect_on(objects[point].borrow());
                } else {
                    for &source in &tree.points[neighbour - leaf_idx] {
                        field[point] += objects[source].borrow().effect_on(objects[point].borrow());
                    }
                }
            }
        }

        // Compute pairwise interactions for contained points
        for (&a, &b) in target.iter().tuple_combinations() {
            field[a] += objects[b].borrow().effect_on(objects[a].borrow());
            field[b] += objects[a].borrow().effect_on(objects[b].borrow());
        }

        // Apply accumulated acceleration on cell to all contained points
        for &point in target {
            field[point] += reactions.inner[quad].acc;
        }
    }
}

const fn ceil_log4(x: usize) -> usize {
    ((x | 1).ilog2() as usize + !x.is_power_of_two() as usize + 1) >> 1
}
