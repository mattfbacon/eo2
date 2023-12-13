use std::cmp::Ordering;
use std::hash::{Hash, Hasher as _};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug)]
pub enum Direction {
	Left,
	Right,
}

#[derive(Clone, Copy, Debug)]
pub enum Mode {
	Simple,
	Random { seed: u64 },
}

#[derive(Clone, Copy, Debug)]
pub struct NextPath {
	pub direction: Direction,
	pub mode: Mode,
}

impl Direction {
	fn for_ordering(self, ordering: Ordering) -> Ordering {
		match self {
			Self::Right => ordering,
			Self::Left => ordering.reverse(),
		}
	}

	fn before<T: Ord + ?Sized>(self, left: &T, right: &T) -> bool {
		self.for_ordering(Ord::cmp(left, right)).is_lt()
	}

	fn after<T: Ord + ?Sized>(self, left: &T, right: &T) -> bool {
		self.for_ordering(Ord::cmp(left, right)).is_gt()
	}
}

trait MakeFindNextKey {
	type Key: Ord + Eq + Clone + Copy + std::fmt::Debug;

	fn for_name(&self, s: &str) -> Self::Key;
}

struct NoKey;

impl MakeFindNextKey for NoKey {
	type Key = ();

	fn for_name(&self, _: &str) -> Self::Key {}
}

struct WithHash {
	seed: u64,
}

pub fn fxhash(v: &(impl Hash + ?Sized)) -> u64 {
	let mut hasher = rustc_hash::FxHasher::default();
	v.hash(&mut hasher);
	hasher.finish()
}

impl MakeFindNextKey for WithHash {
	type Key = u64;

	fn for_name(&self, s: &str) -> Self::Key {
		fxhash(&(self.seed, s))
	}
}

#[derive(Debug, Clone)]
struct HumanCompare<T>(T);

impl<T: AsRef<str>> AsRef<str> for HumanCompare<T> {
	fn as_ref(&self) -> &str {
		self.0.as_ref()
	}
}

impl<T: AsRef<str>> HumanCompare<T> {
	fn as_ref(&self) -> HumanCompare<&str> {
		HumanCompare(self.0.as_ref())
	}
}

impl<T: AsRef<str>, U: AsRef<str>> PartialEq<HumanCompare<U>> for HumanCompare<T> {
	fn eq(&self, other: &HumanCompare<U>) -> bool {
		self.0.as_ref() == other.0.as_ref()
	}
}

impl<T: AsRef<str>> Eq for HumanCompare<T> {}

impl<T: AsRef<str>, U: AsRef<str>> PartialOrd<HumanCompare<U>> for HumanCompare<T> {
	fn partial_cmp(&self, other: &HumanCompare<U>) -> Option<Ordering> {
		Some(natord::compare(self.0.as_ref(), other.0.as_ref()))
	}
}

impl<T: AsRef<str>> Ord for HumanCompare<T> {
	fn cmp(&self, other: &Self) -> Ordering {
		natord::compare(self.0.as_ref(), other.0.as_ref())
	}
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
struct FindNextItem<Key, Name: AsRef<str>> {
	key: Key,
	name: HumanCompare<Name>,
}

struct WithIndex<T> {
	inner: T,
	idx: usize,
}

fn find_next_impl<K: MakeFindNextKey + ?Sized>(
	direction: Direction,
	current_name: &str,
	dir: impl Iterator<Item = String>,
	make_key: &K,
) -> Option<(String, usize)> {
	let mut next_name: Option<WithIndex<FindNextItem<K::Key, String>>> = None;
	let mut wrapped_name: Option<WithIndex<FindNextItem<K::Key, String>>> = None;

	let current_name = FindNextItem {
		key: make_key.for_name(current_name),
		name: HumanCompare(current_name),
	};

	for (idx, this_name) in dir.enumerate() {
		let this_name = FindNextItem {
			key: make_key.for_name(&this_name),
			name: HumanCompare(this_name),
		};

		if wrapped_name.as_ref().map_or(true, |first_name| {
			direction.before(&this_name, &first_name.inner)
		}) {
			wrapped_name = Some(WithIndex {
				inner: this_name.clone(),
				idx,
			});
		}

		if direction.after(
			&FindNextItem {
				key: this_name.key,
				name: this_name.name.as_ref(),
			},
			&current_name,
		) && next_name.as_ref().map_or(true, |next_name| {
			direction.before(&this_name, &next_name.inner)
		}) {
			next_name = Some(WithIndex {
				inner: this_name,
				idx,
			});
		}
	}

	next_name
		.or(wrapped_name)
		.map(|item| (item.inner.name.0, item.idx))
}

#[test]
fn test_find_next_impl() {
	use std::collections::HashSet;

	const FILES: &[&str] = &["a", "b", "c", "d"];

	fn files() -> impl Iterator<Item = String> {
		FILES.iter().map(|&s| s.to_owned())
	}

	for (current_idx, chunk) in FILES.windows(2).enumerate() {
		let &[current, next] = chunk else {
			unreachable!();
		};
		assert_eq!(
			find_next_impl(Direction::Right, current, files(), &NoKey),
			Some((next.into(), current_idx + 1)),
		);
	}
	assert_eq!(
		find_next_impl(Direction::Right, FILES.last().unwrap(), files(), &NoKey),
		Some((FILES.first().copied().unwrap().into(), 0)),
	);

	for (prev_idx, chunk) in FILES.windows(2).enumerate().rev() {
		let &[prev, current] = chunk else {
			unreachable!();
		};
		assert_eq!(
			find_next_impl(Direction::Left, current, files(), &NoKey),
			Some((prev.into(), prev_idx))
		);
	}
	assert_eq!(
		find_next_impl(Direction::Left, FILES.first().unwrap(), files(), &NoKey),
		Some((FILES.last().copied().unwrap().into(), FILES.len() - 1)),
	);

	// fuzz with various seeds
	for _ in 0..20 {
		let random_seed = rand::random();
		let mut current = FILES.first().copied().unwrap().to_owned();
		let mut seen = HashSet::from([current.clone()]);
		let mut seen_idxs = HashSet::from([0]);
		loop {
			let (next, next_idx) = find_next_impl(
				Direction::Right,
				&current,
				files(),
				&WithHash { seed: random_seed },
			)
			.unwrap();
			if next == FILES.first().copied().unwrap() {
				break;
			}
			assert!(seen_idxs.insert(next_idx), "no indexes are repeated");
			assert!(seen.insert(next.clone()), "no files are repeated");
			current = next;
		}
		assert_eq!(seen, files().collect(), "all files are seen");
		assert_eq!(
			seen_idxs,
			(0..FILES.len()).collect(),
			"all indexes are seen"
		);
	}
}

pub fn read_dir_to_find_next_iterator(dir: std::fs::ReadDir) -> impl Iterator<Item = String> {
	dir
		.filter_map(Result::ok)
		.filter(|entry| entry.file_type().map_or(false, |ty| !ty.is_dir()))
		.map(|entry| entry.file_name())
		.filter(|name| image::ImageFormat::from_path(name).is_ok())
		.map(|name| name.to_string_lossy().into_owned())
}

impl NextPath {
	fn find_next(
		self,
		current_name: &str,
		items: impl Iterator<Item = String>,
	) -> Option<(String, usize)> {
		match self.mode {
			Mode::Simple => find_next_impl(self.direction, current_name, items, &NoKey),
			Mode::Random { seed } => {
				find_next_impl(self.direction, current_name, items, &WithHash { seed })
			}
		}
	}
}

pub fn next_in_directory(current_path: &Path, direction: NextPath) -> io::Result<Option<PathBuf>> {
	let parent = current_path.parent().unwrap(/* path must have a parent because it must be a file, though it may be empty. */);
	let current_name = current_path.file_name().unwrap(/* ditto */).to_string_lossy();

	let readable_parent = if parent.as_os_str().is_empty() {
		".".as_ref()
	} else {
		parent
	};

	let next_name = direction.find_next(
		&current_name,
		read_dir_to_find_next_iterator(readable_parent.read_dir()?),
	);

	Ok(next_name.map(|(next_name, _idx)| parent.join(next_name)))
}

pub fn next_in_list<'a>(
	list: impl Iterator<Item = &'a Path>,
	current_path: &Path,
	direction: NextPath,
) -> Option<usize> {
	let current_name = current_path.to_string_lossy();

	let next_name = direction.find_next(
		&current_name,
		list.map(|path| path.to_string_lossy().into_owned()),
	);

	next_name.map(|(_, idx)| idx)
}
