package utils

import (
	"fmt"
	"strconv"
	"strings"

	"github.com/joho/godotenv"
	"github.com/spf13/viper"
)

type Config struct {
	TiingoWSURL   string
	TiingoAPIKey  string
	RedisAddrs    []string
	RedisPassword string
	AppPort       string
	SfnodeID      int64
}

func LoadEnv() (*Config, error) {
	_ = godotenv.Load()
	v := viper.New()

	// Enable ENV variables
	v.AutomaticEnv()

	// Optional: allow ENV names like REDIS_PASSWORD
	v.SetEnvKeyReplacer(strings.NewReplacer(".", "_"))

	addrs := v.GetString("REDIS_ADDRS")
	if addrs == "" {
		return nil, fmt.Errorf("no Redis addresses configured in REDIS_ADDRS")
	}
	nodeID, err := strconv.ParseInt(v.GetString("SNOWFLAKE_NODE_ID"), 10, 64)
	if err != nil {
		return nil, fmt.Errorf("invalid Node ID: %s", err)
	}
	redisAddrs := strings.Split(addrs, ",")
	for i := range redisAddrs {
		redisAddrs[i] = strings.TrimSpace(redisAddrs[i])
	}
	cfg := &Config{
		TiingoWSURL:   v.GetString("TIINGO_WS_URL"),
		TiingoAPIKey:  v.GetString("TIINGO_API_KEY"),
		RedisAddrs:    redisAddrs,
		RedisPassword: v.GetString("REDIS_PASSWORD"),
		AppPort:       v.GetString("APP_PORT"),
		SfnodeID:      nodeID,
	}

	return cfg, nil
}

func ToInterfaceSlice(ss []string) []interface{} {
	out := make([]interface{}, len(ss))
	for i, s := range ss {
		out[i] = s
	}
	return out
}
