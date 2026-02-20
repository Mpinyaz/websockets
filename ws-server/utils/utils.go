package utils

import (
	"fmt"
	"strconv"
	"strings"

	"github.com/joho/godotenv"
	"github.com/spf13/viper"
)

type Config struct {
	RedisAddrs      []string
	RedisPassword   string
	AppPort         string
	SfnodeID        int64
	RmqHost         string
	RmqUsername     string
	RmqPassword     string
	RmqMarketUpdate string
	RmqMarketSubs   string
	RmqPort         int
	InfluxDBURL     string // New: InfluxDB URL
	InfluxDBToken   string // New: InfluxDB Token
	InfluxDBOrg     string // New: InfluxDB Organization
	InfluxDBBucket  string // New: InfluxDB Bucket
}

func LoadEnv() (*Config, error) {
	_ = godotenv.Load()
	v := viper.New()

	// Enable ENV variables
	v.AutomaticEnv()

	v.SetEnvKeyReplacer(strings.NewReplacer(".", "_"))

	addrs := v.GetString("REDIS_ADDRS")
	if addrs == "" {
		return nil, fmt.Errorf("no Redis addresses configured in REDIS_ADDRS")
	}
	nodeID, err := strconv.ParseInt(v.GetString("SNOWFLAKE_NODE_ID"), 10, 64)
	if err != nil {
		return nil, fmt.Errorf("invalid Node ID: %s", err)
	}

	rmqPort, err := strconv.ParseInt(v.GetString("RABBITMQ_STREAM_PORT"), 10, 32)
	if err != nil {
		return nil, fmt.Errorf("invalid Node ID: %s", err)
	}
	redisAddrs := strings.Split(addrs, ",")
	for i := range redisAddrs {
		redisAddrs[i] = strings.TrimSpace(redisAddrs[i])
	}
	cfg := &Config{
		RedisAddrs:      redisAddrs,
		RedisPassword:   v.GetString("REDIS_PASSWORD"),
		AppPort:         v.GetString("MDWS_APP_PORT"),
		SfnodeID:        nodeID,
		RmqHost:         v.GetString("RABBITMQ_ADVERTISED_HOST"),
		RmqUsername:     v.GetString("RABBITMQ_DEFAULT_USER"),
		RmqPassword:     v.GetString("RABBITMQ_DEFAULT_PASS"),
		RmqMarketUpdate: v.GetString("MDWS_FEED_STREAM"),
		RmqMarketSubs:   v.GetString("MDWS_SUBSCRIBE_STREAM"),
		RmqPort:         int(rmqPort),
		InfluxDBURL:     v.GetString("INFLUXDB_URL"),
		InfluxDBToken:   v.GetString("INFLUXDB_TOKEN"),
		InfluxDBOrg:     v.GetString("INFLUXDB_ORG"),
		InfluxDBBucket:  v.GetString("INFLUXDB_BUCKET"),
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
