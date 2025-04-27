create table if not exists trades 
(
    id uuid primary key not null default uuid_generate_v4(),
    trade_no int8 not null,
    security_id uuid not null references public.securities(id) on delete cascade,
    trade_datetime timestamp without time zone not null,
    price float4 not null default 0.0,
    quantity int4 not null default 0, 
    value float4 not null default 0.0,
    buysell varchar(1) not null
);
