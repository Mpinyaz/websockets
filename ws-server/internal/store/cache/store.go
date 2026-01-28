// Package cache provides Redis-backed storage for websocket client subscriptions.
//
// It supports:
//   - Client → symbols lookups
//   - Symbol → clients lookups
//   - Redis Cluster-safe key layouts
package cache

import (
	"context"
	"errors"
	"log"

	. "websockets/utils"

	"github.com/go-redis/redis/v8"
)

type RedisStore struct {
	Client *redis.ClusterClient
}

var ErrNil = errors.New("no matching record found in redis database")

func NewRedisClient(cfg *Config) (*RedisStore, error) {
	client := redis.NewClusterClient(&redis.ClusterOptions{
		Addrs: cfg.RedisAddrs,
		// Password: cfg.RedisPassword,
		PoolSize: 50,
	})

	if err := client.Ping(context.Background()).Err(); err != nil {
		return nil, err
	}

	return &RedisStore{
		Client: client,
	}, nil
}

func (r *RedisStore) Close() error {
	return r.Client.Close()
}

func (r *RedisStore) Run() {
	ctx := context.Background()

	status, err := r.Client.Ping(ctx).Result()
	if err != nil {
		log.Fatalln("Redis cluster connection failed:", err)
	}

	log.Println("Redis cluster status:", status)
}
