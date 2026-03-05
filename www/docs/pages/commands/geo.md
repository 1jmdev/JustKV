# GEO Commands

GEO commands store longitude/latitude points in sorted sets and query by distance or area.

## Write and Basic Read

```
GEOADD key [NX | XX] [CH] longitude latitude member [longitude latitude member ...]
GEOPOS key member [member ...]
GEODIST key member1 member2 [m | km | ft | mi]
GEOHASH key member [member ...]
```

Add positions, read coordinates, compute distance, and return geohashes.

**Complexity:** O(log N) per added point, O(1) per lookup

```bash
GEOADD cities 13.361389 38.115556 palermo 15.087269 37.502669 catania
GEOPOS cities palermo catania
GEODIST cities palermo catania km
GEOHASH cities palermo
```

## Radius Queries

```
GEORADIUS key longitude latitude radius m | km | ft | mi [WITHCOORD] [WITHDIST] [WITHHASH] [COUNT count [ANY]] [ASC | DESC] [STORE key] [STOREDIST key]
GEORADIUS_RO key longitude latitude radius m | km | ft | mi [WITHCOORD] [WITHDIST] [WITHHASH] [COUNT count [ANY]] [ASC | DESC]
GEORADIUSBYMEMBER key member radius m | km | ft | mi [WITHCOORD] [WITHDIST] [WITHHASH] [COUNT count [ANY]] [ASC | DESC] [STORE key] [STOREDIST key]
GEORADIUSBYMEMBER_RO key member radius m | km | ft | mi [WITHCOORD] [WITHDIST] [WITHHASH] [COUNT count [ANY]] [ASC | DESC]
```

Find members within a radius from a coordinate or existing member.

**Complexity:** O(N+log M)

```bash
GEORADIUS cities 15 37 200 km WITHDIST
GEORADIUSBYMEMBER cities palermo 100 km WITHCOORD
```

## Search and Store

```
GEOSEARCH key FROMLONLAT longitude latitude | FROMMEMBER member BYRADIUS radius m | km | ft | mi | BYBOX width height m | km | ft | mi [ASC | DESC] [COUNT count [ANY]] [WITHCOORD] [WITHDIST] [WITHHASH]
GEOSEARCHSTORE destination source FROMLONLAT longitude latitude | FROMMEMBER member BYRADIUS radius m | km | ft | mi | BYBOX width height m | km | ft | mi [ASC | DESC] [COUNT count [ANY]] [STOREDIST]
```

Modern area search APIs and stored search results.

**Complexity:** O(N+log M)

```bash
GEOSEARCH cities FROMLONLAT 15 37 BYRADIUS 200 km WITHDIST
GEOSEARCHSTORE nearby cities FROMMEMBER palermo BYRADIUS 100 km
```
