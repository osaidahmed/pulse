<?php

function too_many_args(int $a, int $b, int $c, int $d, int $e, int $f): int {
    return $a + $b + $c + $d + $e + $f;
}

class Service {
    public function __construct(
        private readonly mixed $repo,
        private readonly mixed $logger,
        private readonly mixed $cache,
        private readonly mixed $mailer,
        private readonly mixed $queue,
        private readonly mixed $config
    ) {}
}
