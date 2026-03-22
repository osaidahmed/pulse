<?php

namespace App\Api;

class ApiService {
    public function __construct(
        private readonly mixed $client,
        private readonly mixed $cache
    ) {}

    public function get(string $url): array {
        $cached = $this->cache->get($url);
        if ($cached !== null) {
            return $cached;
        }
        $response = $this->client->request('GET', $url);
        $this->cache->set($url, $response);
        return $response;
    }

    public function post(string $url, array $data): array {
        return $this->client->request('POST', $url, $data);
    }

    public function delete(string $url): bool {
        $this->cache->forget($url);
        return $this->client->request('DELETE', $url) !== null;
    }
}
