package cache

import (
	"context"
	"fmt"
	"strconv"

	. "websockets/internal/models"

	"github.com/redis/go-redis/v9"
)

type ClientStore struct {
	rdb *RedisStore
}

// NewClientStore creates a new client store for a standalone Redis client
func NewClientStore(rdb *RedisStore) *ClientStore {
	return &ClientStore{rdb: rdb}
}

func clientKey(clientID string, asset AssetClass) string {
	return fmt.Sprintf("client:{%s}:%s", clientID, asset.String())
}

func symbolKey(asset AssetClass, symbol string) string {
	return fmt.Sprintf("symbol:{%s}:%s", asset.String(), symbol)
}

func toInterfaceSlice(ss []string) []interface{} {
	out := make([]interface{}, len(ss))
	for i, s := range ss {
		out[i] = s
	}
	return out
}

func (c *ClientStore) addClientToSymbols(
	ctx context.Context,
	clientID string,
	asset AssetClass,
	symbols []string,
) error {
	if len(symbols) == 0 {
		return nil
	}

	_, err := c.rdb.client.Pipelined(ctx, func(p redis.Pipeliner) error {
		for _, symbol := range symbols {
			p.SAdd(ctx, symbolKey(asset, symbol), clientID)
		}
		return nil
	})

	return err
}

func (c *ClientStore) removeClientFromSymbols(
	ctx context.Context,
	clientID string,
	asset AssetClass,
	symbols []string,
) error {
	if len(symbols) == 0 {
		return nil
	}

	_, err := c.rdb.client.Pipelined(ctx, func(p redis.Pipeliner) error {
		for _, symbol := range symbols {
			p.SRem(ctx, symbolKey(asset, symbol), clientID)
		}
		return nil
	})

	return err
}

func (c *ClientStore) SetClientSubs(sub ClientSub) error {
	ctx := context.Background()

	clientID := strconv.FormatInt(sub.ID.Int64(), 10)
	process := func(asset AssetClass, newSymbols *[]string) error {
		if newSymbols == nil {
			return nil
		}

		ck := clientKey(clientID, asset)

		oldSymbols, err := c.rdb.client.SMembers(ctx, ck).Result()
		if err != nil && err != redis.Nil {
			return err
		}

		if err := c.removeClientFromSymbols(ctx, clientID, asset, oldSymbols); err != nil {
			return err
		}

		if err := c.rdb.client.Del(ctx, ck).Err(); err != nil {
			return err
		}

		if len(*newSymbols) > 0 {
			if err := c.rdb.client.SAdd(ctx, ck, toInterfaceSlice(*newSymbols)...).Err(); err != nil {
				return err
			}
			return c.addClientToSymbols(ctx, clientID, asset, *newSymbols)
		}

		return nil
	}

	if err := process(Forex, sub.Forex); err != nil {
		return err
	}
	if err := process(Equity, sub.Equity); err != nil {
		return err
	}
	if err := process(Crypto, sub.Crypto); err != nil {
		return err
	}

	return nil
}

func (c *ClientStore) PatchClientSub(asset AssetClass, sub ClientSub) error {
	ctx := context.Background()
	clientID := sub.ID.String()

	var symbols *[]string
	switch asset {
	case Forex:
		symbols = sub.Forex
	case Equity:
		symbols = sub.Equity
	case Crypto:
		symbols = sub.Crypto
	}

	if symbols == nil || len(*symbols) == 0 {
		return nil
	}

	ck := clientKey(clientID, asset)

	if err := c.rdb.client.SAdd(ctx, ck, toInterfaceSlice(*symbols)...).Err(); err != nil {
		return err
	}

	return c.addClientToSymbols(ctx, clientID, asset, *symbols)
}

func (c *ClientStore) RemoveClientSubs(clientID int64) error {
	ctx := context.Background()
	id := strconv.FormatInt(clientID, 10)

	assets := []AssetClass{
		Forex,
		Equity,
		Crypto,
	}

	_, err := c.rdb.client.Pipelined(ctx, func(p redis.Pipeliner) error {
		for _, asset := range assets {
			ck := clientKey(id, asset)

			symbols, err := c.rdb.client.SMembers(ctx, ck).Result()
			if err != nil && err != redis.Nil {
				return err
			}

			for _, symbol := range symbols {
				p.SRem(ctx, symbolKey(asset, symbol), id)
			}

			p.Del(ctx, ck)
		}
		return nil
	})

	return err
}

func (c *ClientStore) RemoveClientSubsByAsset(
	clientID int64,
	asset AssetClass,
) error {
	ctx := context.Background()
	id := strconv.FormatInt(clientID, 10)

	ck := clientKey(id, asset)

	symbols, err := c.rdb.client.SMembers(ctx, ck).Result()
	if err != nil && err != redis.Nil {
		return err
	}

	_, err = c.rdb.client.Pipelined(ctx, func(p redis.Pipeliner) error {
		for _, symbol := range symbols {
			p.SRem(ctx, symbolKey(asset, symbol), id)
		}
		p.Del(ctx, ck)
		return nil
	})

	return err
}
