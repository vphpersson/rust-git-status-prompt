extern crate git2;

use git2::{Branch, Repository, Status, StatusOptions};

struct Counts {
    changed: usize,
    conflicts: usize,
    staged: usize,
    untracked: usize,
}

fn get_status_counts(repo: &Repository) -> Counts {
    let mut counts = Counts {changed: 0, conflicts: 0, staged: 0, untracked: 0};

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);

    let statuses = repo
        .statuses(Some(&mut opts))
        .expect("Unable to gather status information.")
    ;

    let mut staged = Status::empty();
    staged.insert(Status::INDEX_NEW);
    staged.insert(Status::INDEX_MODIFIED);
    staged.insert(Status::INDEX_DELETED);
    staged.insert(Status::INDEX_RENAMED);
    staged.insert(Status::INDEX_TYPECHANGE);

    let mut changed = Status::empty();
    changed.insert(Status::WT_MODIFIED);
    changed.insert(Status::WT_DELETED);
    changed.insert(Status::WT_RENAMED);
    changed.insert(Status::WT_TYPECHANGE);

    for entry in statuses.iter() {
        match entry.status() {
            s if s.intersects(staged) => counts.staged += 1,
            s if s.intersects(changed) => counts.changed += 1,
            s if s.contains(Status::CONFLICTED) => counts.conflicts += 1,
            s if s.contains(Status::WT_NEW) => counts.untracked += 1,
            _ => (),
        }
    }

    counts
}

fn ahead_behind(repo: &Repository) -> (usize, usize) {
    let default = (0, 0);

    let head = match repo.head() {
        Ok(head) => Some(head).unwrap(),
        Err(_) => return default,
    };
    let local_oid = head.target().expect("Unable to determine Oid of head.");

    let upstream_branch = Branch::wrap(head);
    let upstream = match upstream_branch.upstream() {
        Ok(u) => u,
        Err(_) => return default,
    };
    let upstream_oid = match upstream.into_reference().target() {
        Some(u) => u,
        None => return default,
    };

    match repo.graph_ahead_behind(local_oid, upstream_oid) {
        Ok(ab) => ab,
        Err(_) => default,
    }
}

fn get_branch_name(repo: &Repository) -> String {
    let default = String::from("master");

    let head = match repo.head() {
        Ok(head) => head,
        Err(_) => match repo.find_reference("HEAD") {
            Ok(h) => h,
            Err(_) => return default,
        },
    };

    if head.is_branch() {
        // easy case: a checked out branch, give us the name of that branch
        return String::from(
            Branch::wrap(head)
                .name()
                .expect("Unable to determine name of branch.")
                .unwrap()
        );
    }

    let config = repo
        .config()
        .expect("Unable to open config for this repository.");
    let hash_length = match config.get_i32("core.abbrev") {
        Ok(l) => l + 1,
        Err(_) => 9
    };

    match head.symbolic_target() {
        // this is an unborn branch probably? and/or like a repo with no
        // commits? so say it's master. who knows man git is weird
        Some(_) => default,
        // this is anything else, generally a specific commit i guess?
        // like `git checkout HEAD~1`
        None => {
            let mut commit = format!(":{}", head.target().unwrap());
            commit.truncate(hash_length as usize);
            commit
        }
    }
}

fn main() {
    let repo = match Repository::discover(".") {
        Ok(repo) => repo,
        Err(_) => return
    };

    let counts = get_status_counts(&repo);
    let (num_ahead, num_behind) = ahead_behind(&repo);
    let branch_name = get_branch_name(&repo);

    println!(
        "{} {} {} {} {} {} {}",
        branch_name,
        num_ahead,
        num_behind,
        counts.staged,
        counts.conflicts,
        counts.changed,
        counts.untracked
    );
}
