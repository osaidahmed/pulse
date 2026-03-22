<?php

namespace App\Services;

class OrderService {
    public function __construct(
        private readonly mixed $repo,
        private readonly mixed $logger
    ) {}

    public function create(array $data): array {
        $this->logger->info("Creating order");
        return $this->repo->save($data);
    }

    public function update(int $id, array $data): array {
        $this->logger->info("Updating order " . $id);
        return $this->repo->update($id, $data);
    }

    public function delete(int $id): bool {
        $this->logger->info("Deleting order " . $id);
        return $this->repo->delete($id);
    }

    public function find(int $id): ?array {
        return $this->repo->find($id);
    }

    public function findAll(): array {
        return $this->repo->findAll();
    }

    public function process(array $order): array {
        if ($order['status'] === 'pending') {
            $order['status'] = 'processing';
        }
        return $order;
    }

    public function validate(array $data): bool {
        if (empty($data['name'])) {
            return false;
        }
        if (empty($data['email'])) {
            return false;
        }
        return true;
    }

    public function ship(int $id): bool {
        $order = $this->repo->find($id);
        if ($order === null) {
            return false;
        }
        $this->repo->update($id, ['status' => 'shipped']);
        return true;
    }

    public function cancel(int $id): bool {
        $order = $this->repo->find($id);
        if ($order === null) {
            return false;
        }
        $this->repo->update($id, ['status' => 'cancelled']);
        return true;
    }

    public function refund(int $id, float $amount): bool {
        $order = $this->repo->find($id);
        if ($order === null) {
            return false;
        }
        return $amount > 0;
    }

    public function archive(int $id): bool {
        return $this->repo->archive($id);
    }
}
