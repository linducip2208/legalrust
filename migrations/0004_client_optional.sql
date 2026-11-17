-- 0004: matters may start without a linked client (intake-first flow)
ALTER TABLE matters MODIFY client_id BIGINT UNSIGNED NULL;
