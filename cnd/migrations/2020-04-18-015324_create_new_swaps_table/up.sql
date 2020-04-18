-- Your SQL goes here

CREATE TABLE swaps
(
    id INTEGER                   NOT NULL PRIMARY KEY,
    local_swap_id                UNIQUE NOT NULL
);

CREATE table han_bitcoin
(
    id INTEGER                   NOT NULL PRIMARY KEY,
    local_swap_id                UNIQUE NOT NULL,
    FOREIGN KEY (local_swap_id)  REFERENCES swaps(local_swap_id)
);


create table han_ethereum
(
    id INTEGER                   NOT NULL PRIMARY KEY,
    local_swap_id                UNIQUE NOT NULL,
    FOREIGN KEY (local_swap_id)  REFERENCES swaps(local_swap_id)
);
