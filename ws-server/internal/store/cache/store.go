// Package cache provides Redis-backed storage for websocket client subscriptions.
//
// It supports:
//   - Client → symbols lookups
//   - Symbol → clients lookups
//   - Works with standalone Redis or Redis Cluster (for dev or production)
//
// TODO: implement Redis cluster instance
package cache

import (
	"context"
	"log"
	"time"

	. "websockets/utils"

	"github.com/redis/go-redis/v9"
)

type RedisStore struct {
	client redis.Cmdable
	// Keep a reference to the actual client so we can close it
	rawClient interface{}
}

func NewRedisClient(cfg *Config) (*RedisStore, error) {
	var client redis.Cmdable
	var raw interface{}

	if len(cfg.RedisAddrs) == 1 {
		// Standalone Redis
		c := redis.NewClient(&redis.Options{
			Addr:         cfg.RedisAddrs[0],
			PoolSize:     50,
			DialTimeout:  30 * time.Second,
			ReadTimeout:  30 * time.Second,
			WriteTimeout: 30 * time.Second,
		})
		client = c
		raw = c
	} else {
		// Redis Cluster
		c := redis.NewClusterClient(&redis.ClusterOptions{
			Addrs:          cfg.RedisAddrs,
			PoolSize:       50,
			DialTimeout:    30 * time.Second,
			ReadTimeout:    30 * time.Second,
			WriteTimeout:   30 * time.Second,
			RouteByLatency: true,
		})
		client = c
		raw = c
	}

	ctx := context.Background()
	if err := client.Ping(ctx).Err(); err != nil {
		return nil, err
	}

	return &RedisStore{
		client:    client,
		rawClient: raw,
	}, nil
}

// Close closes the underlying client
func (r *RedisStore) Close() error {
	switch c := r.rawClient.(type) {
	case *redis.Client:
		return c.Close()
	case *redis.ClusterClient:
		return c.Close()
	default:
		return nil
	}
}

func (r *RedisStore) Run() {
	ctx := context.Background()

	status, err := r.client.Ping(ctx).Result()
	if err != nil {
		log.Fatalln("Redis connection failed:", err)
	}

	log.Println("Redis status:", status)
}
