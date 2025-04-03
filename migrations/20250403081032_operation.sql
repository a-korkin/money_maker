create table if not exists operations
(
    id uuid primary key not null default uuid_generate_v4(),
    attempt uuid not null,
    operation_type varchar(255) not null,
    security_id uuid not null references public.securities(id) on delete cascade,
    count integer not null default 0,
    price decimal not null default 0,
    commission decimal not null default 0,
    time_at timestamp without time zone not null default now(),
    sum_before decimal not null default 0,
    sum_after decimal not null default 0,
    prev uuid
);
