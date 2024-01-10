import sqlite3
from pathlib import Path
from argparse import ArgumentParser, FileType
from subprocess import check_output
from shlex import split as shellsplit
from contextlib import contextmanager, closing
from dataclasses import dataclass


class UniqueConstraintError(Exception):
    """Value for table field already exists."""


def git_file_name(hash: str):
    return (
        check_output(f"git ls-tree -r @ | awk '/{hash}/{{print $NF}}'", shell=True)
        .decode()
        .strip()
    )


@contextmanager
def db_ctx():
    dbp = Path(Path.home(), "exomem.d", "tag.db")
    with closing(sqlite3.connect(dbp)) as con:
        cur = con.cursor()
        yield cur
        con.commit()


def insert(statement: str):
    with db_ctx() as db:
        try:
            db.execute(f"insert into {statement}")
        except sqlite3.IntegrityError as error:
            msg = str(error)
            if msg.startswith("UNIQUE constraint failed:"):
                file_name = git_file_name(hash)
                raise UniqueConstraintError(f"{error}: {hash}, {file_name}") from error
        db.execute("select last_insert_rowid()")
        (result,) = db.fetchone()
    return result


def select(statement: str):
    with db_ctx() as db:
        db.execute(f"select {statement}")
        results = db.fetchall()
    return results


@dataclass
class Relation:
    _id: int = 0

    def insert(self):
        """Insert a new row and return the id."""

    def select(self):
        """Return id of a existing row."""

    def id(self):
        if self._id:
            return self._id
        try:
            self._id = self.insert()
        except UniqueConstraintError as e:
            self._id = self.select()[0][0]
        return self._id


@dataclass
class Blob(Relation):
    hash: str = ""

    def insert(self):
        return insert(f"blobs (hash) values ('{self.hash}')")

    def select(self):
        return select(f"oid from blobs where hash = '{self.hash}'")


@dataclass
class Tag(Relation):
    name: str = ""

    def insert(self):
        return insert(f"tags (name) values ('{self.name}')")

    def select(self):
        return select(f"oid from tags where name = '{self.name}'")


@dataclass
class BlobTag(Relation):
    blob_id: int = 0
    tag_id: int = 0

    def insert(self):
        return insert(
            f"blob_tags (blob_id, tag_id) values ({self.blob_id}, {self.tag_id})"
        )

    def select(self):
        return select(
            f"oid from blob_tags where blob_id = {self.blob_id} and tag_id = {self.tag_id}"
        )


def print_tags_per_file(tag_names):
    quoted_names = [f"'{name}'" for name in tag_names]
    values = ",".join(quoted_names)
    blobs_with_tags_query = f"""
    with _tags as (select rowid from tags where name in ({values}))
    select distinct blobs.hash from blobs
    inner join _tags on blob_tags.tag_id = _tags.rowid
    inner join blob_tags on blob_tags.blob_id = blobs.rowid;
    """
    with db_ctx() as db:
        db.execute(blobs_with_tags_query)
        blob_regex = "|".join([_id[0] for _id in db.fetchall()])
        out = (
            check_output(
                f"git ls-tree @ | awk '/{blob_regex}/{{print $3 \",\" $4}}'", shell=True
            )
            .decode()
            .strip()
        )
    for line in out.splitlines():
        hash, file_name = line.split(",", maxsplit=1)
        blob_id = select(f"oid from blobs where hash = '{hash}'")[0][0]
        tags_per_blob_query = f"""
        with _blob_tags as (select blob_id, tag_id from blob_tags where blob_id = {blob_id})
        select tags.name from tags
        inner join _blob_tags on _blob_tags.tag_id = tags.rowid;
        """
        with db_ctx() as db:
            db.execute(tags_per_blob_query)
            tags = ", ".join([tag[0] for tag in db.fetchall()])
        print(f"{file_name}: {tags}")


def parse_args():
    argp = ArgumentParser()
    argp.add_argument("-f", "--file", type=FileType("r"))
    argp.add_argument("-t", "--tags", type=str, nargs="+")
    return argp.parse_args(), argp


def main():
    args, argp = parse_args()
    if args.file and not args.tags:
        print("TAGS required.")
        argp.print_usage()
        return
    elif args.file and args.tags:
        hash = (
            check_output(shellsplit(f"git hash-object {args.file.name}"))
            .decode()
            .strip()
        )
        blob = Blob(hash=hash)
        for name in args.tags:
            tag = Tag(name=name)
            blob_tag = BlobTag(blob_id=blob.id(), tag_id=tag.id())
            print(blob_tag.id())
    elif args.tags:
        print_tags_per_file(args.tags)
    else:
        argp.print_help()


if __name__ == "__main__":
    main()
