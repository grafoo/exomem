create table if not exists blobs (
	hash text,
	constraint unique_object_hash unique(hash)
);

create table if not exists tags (
	name text,
	constraint unique_tag_name unique(name)
);

create table if not exists blob_tags (
	blob_id integer,
	tag_id integer,
	foreign key (blob_id) references blobs (rowid),
	foreign key (tag_id) references tags (rowid),
	constraint unique_object_tag_row unique(blob_id, tag_id)
);
