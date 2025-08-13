import React, { useEffect, useMemo, useState } from 'react';
import { api } from '../services/api';
import { Fleet, Position, ResourceType, StarSystem, Player, Resource } from '../types/game';
import { ShipMarketModal } from './ShipMarketModal';

type MarketTab = 'trade' | 'compare';

interface MarketViewProps {
    selectedFleet: Fleet | null;
}

function distance3d(a: Position, b: Position): number {
    const dx = a.x - b.x; const dy = a.y - b.y; const dz = a.z - b.z;
    return Math.sqrt(dx*dx + dy*dy + dz*dz);
}

export const MarketView: React.FC<MarketViewProps> = ({ selectedFleet }) => {
    const [active, setActive] = useState<MarketTab>('trade');
    const [system, setSystem] = useState<StarSystem | null>(null);
    const [planetIndex, setPlanetIndex] = useState<number | null>(null);
    const [market, setMarket] = useState<any | null>(null);
    const [shipMarket, setShipMarket] = useState<any | null>(null);
    const [galaxy, setGalaxy] = useState<StarSystem[]>([]);
    const [loading, setLoading] = useState<boolean>(false);
    const [error, setError] = useState<string | null>(null);
    const [qty, setQty] = useState<number>(1);
    const [distMode, setDistMode] = useState<'first' | 'even' | 'selected'>('first');
    const [player, setPlayer] = useState<Player | null>(null);
    const [selectedResource, setSelectedResource] = useState<Resource | null>(null);
    // Ship selection for viewing stock
    const [viewShipIndex, setViewShipIndex] = useState<number>(-1); // -1 => All ships
    // Local snapshot of the fleet so we can refresh cargo after trades
    const [fleetSnapshot, setFleetSnapshot] = useState<Fleet | null>(selectedFleet);
    const [showShipModal, setShowShipModal] = useState<boolean>(false);
    // Compare tab selections and cache of markets
    const [compareSelected, setCompareSelected] = useState<string[]>([]); // key: `${sysId}-${planetIdx}`
    const [compareMarkets, setCompareMarkets] = useState<Record<string, any>>({});
    const [mapHalf, setMapHalf] = useState<number>(0);
    const [compareSearch, setCompareSearch] = useState<string>("");

    const planetKey = (sysId: number, planetIdx: number) => `${sysId}-${planetIdx}`;
    const isSelectedForCompare = (sysId: number, planetIdx: number) => compareSelected.includes(planetKey(sysId, planetIdx));
    const toggleCompareSelect = async (sysId: number, planetIdx: number) => {
        const key = planetKey(sysId, planetIdx);
        setCompareSelected(prev => prev.includes(key) ? prev.filter(k => k !== key) : [...prev, key]);
        // Lazy load market if not present
        if (!compareMarkets[key]) {
            try {
                const m = await api.getPlanetMarket(sysId, planetIdx);
                setCompareMarkets(prev => ({ ...prev, [key]: m }));
            } catch (e) {
                setError(e instanceof Error ? e.message : 'Failed to load comparison market');
            }
        }
    };

    // Distance helpers mirroring backend movement breakdown
    const pointInCube = (p: Position, center: Position, half: number): boolean => (
        p.x >= center.x - half && p.x <= center.x + half &&
        p.y >= center.y - half && p.y <= center.y + half &&
        p.z >= center.z - half && p.z <= center.z + half
    );

    const lineCubeIntersection = (p0: Position, p1: Position, center: Position, half: number): { entry: Position; exit: Position } | null => {
        const min = { x: center.x - half, y: center.y - half, z: center.z - half };
        const max = { x: center.x + half, y: center.y + half, z: center.z + half };
        const d = { x: p1.x - p0.x, y: p1.y - p0.y, z: p1.z - p0.z };
        let tmin = 0; let tmax = 1;
        const upd = (p0c: number, dc: number, minc: number, maxc: number) => {
            if (Math.abs(dc) < 1e-9) { if (p0c < minc || p0c > maxc) return false; }
            else { let t1 = (minc - p0c) / dc; let t2 = (maxc - p0c) / dc; if (t1 > t2) { const tmp = t1; t1 = t2; t2 = tmp; } if (t1 > tmin) tmin = t1; if (t2 < tmax) tmax = t2; if (tmin > tmax) return false; }
            return true;
        };
        if (!upd(p0.x, d.x, min.x, max.x)) return null;
        if (!upd(p0.y, d.y, min.y, max.y)) return null;
        if (!upd(p0.z, d.z, min.z, max.z)) return null;
        const clamp = (t: number) => t < 0 ? 0 : t > 1 ? 1 : t;
        const tEntry = clamp(tmin), tExit = clamp(tmax);
        const entry = { x: Math.round(p0.x + d.x * tEntry), y: Math.round(p0.y + d.y * tEntry), z: Math.round(p0.z + d.z * tEntry) };
        const exit = { x: Math.round(p0.x + d.x * tExit), y: Math.round(p0.y + d.y * tExit), z: Math.round(p0.z + d.z * tExit) };
        return { entry, exit };
    };

    const computeTravelDistanceToPlanet = (fleet: Fleet, sys: StarSystem, planetIdx: number): number => {
        const planet = sys.planets[planetIdx];
        const planetGal: Position = { x: sys.position.x + planet.position.x, y: sys.position.y + planet.position.y, z: sys.position.z + planet.position.z };
        const half = mapHalf || 0;
        if (half <= 0) return distance3d(fleet.position, planetGal);
        const scale = 1.0 / half; // in-system scaling only
        if (fleet.current_system_id != null) {
            if (fleet.current_system_id === sys.id) {
                // Pure in-system move
                const lp = fleet.local_position || { x: 0, y: 0, z: 0 } as Position;
                const local = Math.sqrt((lp.x - planet.position.x) ** 2 + (lp.y - planet.position.y) ** 2 + (lp.z - planet.position.z) ** 2);
                return local * scale;
            }
            // System → galaxy → system
            const currentSys = galaxy.find(s => s.id === fleet.current_system_id);
            if (!currentSys) return distance3d(fleet.position, planetGal);
            // Local exit scaled
            const start = fleet.position;
            const out = lineCubeIntersection(start, sys.position, currentSys.position, half);
            const exitPoint = out ? out.exit : start;
            const inExit = distance3d(start, exitPoint) * scale;
            // Deep segment as center-to-center distance
            const deep = distance3d(currentSys.position, sys.position);
            // Entry-to-planet scaled
            const inEntry = distance3d(sys.position, planetGal) * scale;
            return inExit + deep + inEntry;
        } else {
            // From deep space: deep to target system center, then scaled local to planet
            const deep = distance3d(fleet.position, sys.position);
            const inEntry = distance3d(sys.position, planetGal) * scale;
            return deep + inEntry;
        }
    };

    // Load system + co-located planet
    useEffect(() => {
        const load = async () => {
            setError(null);
            setMarket(null);
            setShipMarket(null);
            setSystem(null);
            setPlanetIndex(null);
            if (!selectedFleet || selectedFleet.current_system_id == null) return;
            try {
                setLoading(true);
                const sys = await api.getStarSystem(selectedFleet.current_system_id);
                setSystem(sys);
                // Find planet by exact local position match
                const lp = selectedFleet.local_position;
                if (lp) {
                    const idx = sys.planets.findIndex(p => p.position.x === lp.x && p.position.y === lp.y && p.position.z === lp.z);
                    if (idx >= 0) {
                        setPlanetIndex(idx);
                        // Load settings → player for credits/inventory
                        const settings = await api.getGameSettings();
                        const [mkt, smkt, playerData] = await Promise.all([
                            api.getPlanetMarket(sys.id, idx),
                            api.getPlanetShipMarket(sys.id, idx).then(r => r.data),
                            api.getPlayer(settings.player_name),
                        ]);
                        setMarket(mkt);
                        setShipMarket(smkt);
                        setPlayer(playerData);
                        setMapHalf(settings.map_width);
                        // Also refresh the player's fleets so cargo counts stay current
                        try {
                            const fleetsResp = await api.getPlayerFleets();
                            const freshFleet = fleetsResp.data.find(f => f.name === selectedFleet.name) || null;
                            setFleetSnapshot(freshFleet ?? selectedFleet);
                        } catch (_) {
                            setFleetSnapshot(selectedFleet);
                        }
                    }
                }
            } catch (e) {
                setError(e instanceof Error ? e.message : 'Failed to load market');
            } finally {
                setLoading(false);
            }
        };
        load();
    }, [selectedFleet?.name, selectedFleet?.current_system_id, selectedFleet?.local_position?.x, selectedFleet?.local_position?.y, selectedFleet?.local_position?.z]);

    // Load galaxy and settings once (for compare + scaling)
    useEffect(() => {
        const loadGalaxy = async () => {
            try {
                const [g, settings] = await Promise.all([
                    api.getGalaxyMap(),
                    api.getGameSettings(),
                ]);
                setGalaxy(g);
                setMapHalf(settings.map_width);
            } catch (e) {
                // ignore compare errors initially
            }
        };
        loadGalaxy();
    }, []);

    const planetAtFleet = useMemo(() => {
        if (!system || planetIndex == null) return null as any;
        return system.planets[planetIndex] || null;
    }, [system, planetIndex]);

    const handleBuy = async (resource: ResourceType) => {
        if (!system || planetIndex == null) return;
        try {
            setLoading(true);
            // If user chose "selected" ship, translate to specific allocation API with a single entry
            if (distMode === 'selected' && viewShipIndex >= 0) {
                await api.buyResource(system.id, planetIndex, resource, qty, selectedFleet?.name, 'specific', [{ ship_index: viewShipIndex, quantity: qty }]);
            } else {
                await api.buyResource(system.id, planetIndex, resource, qty, selectedFleet?.name, distMode as any);
            }
            const m = await api.getPlanetMarket(system.id, planetIndex);
            setMarket(m);
            // refresh player for credits/inventory
            const settings = await api.getGameSettings();
            setPlayer(await api.getPlayer(settings.player_name));
            // refresh fleet cargo snapshot
            try {
                const fleetsResp = await api.getPlayerFleets();
                const freshFleet = fleetsResp.data.find(f => f.name === selectedFleet?.name) || null;
                setFleetSnapshot(freshFleet ?? fleetSnapshot);
            } catch (_) {}
        } catch (e) {
            setError(e instanceof Error ? e.message : 'Buy failed');
        } finally {
            setLoading(false);
        }
    };

    const handleSell = async (resource: ResourceType) => {
        if (!system || planetIndex == null) return;
        try {
            setLoading(true);
            if (distMode === 'selected' && viewShipIndex >= 0) {
                await api.sellResource(system.id, planetIndex, resource, qty, selectedFleet?.name, 'specific', [{ ship_index: viewShipIndex, quantity: qty }]);
            } else {
                await api.sellResource(system.id, planetIndex, resource, qty, selectedFleet?.name, distMode as any);
            }
            const m = await api.getPlanetMarket(system.id, planetIndex);
            setMarket(m);
            const settings = await api.getGameSettings();
            setPlayer(await api.getPlayer(settings.player_name));
            // refresh fleet cargo snapshot
            try {
                const fleetsResp = await api.getPlayerFleets();
                const freshFleet = fleetsResp.data.find(f => f.name === selectedFleet?.name) || null;
                setFleetSnapshot(freshFleet ?? fleetSnapshot);
            } catch (_) {}
        } catch (e) {
            setError(e instanceof Error ? e.message : 'Sell failed');
        } finally {
            setLoading(false);
        }
    };

    // No per-ship allocations UI anymore. When "Selected" is chosen, we dispatch to backend using that single ship.

    // Helper to compute stock for a resource from current fleet snapshot
    const getStockForResource = (resourceType: string): number => {
        const fleetRef = fleetSnapshot || selectedFleet;
        if (!fleetRef) return 0;
        if (viewShipIndex >= 0) {
            const ship = fleetRef.ships[viewShipIndex];
            if (!ship) return 0;
            return ship.cargo.find(c => c.resource_type === resourceType)?.quantity ?? 0;
        }
        // All ships
        return fleetRef.ships.reduce((sum, ship) => {
            const q = ship.cargo.find(c => c.resource_type === resourceType)?.quantity ?? 0;
            return sum + q;
        }, 0);
    };

    // Capacity helpers
    const getCapacityInfo = () => {
        const fleetRef = fleetSnapshot || selectedFleet;
        if (!fleetRef) return { current: 0, capacity: 0, remaining: 0 };
        if (viewShipIndex >= 0) {
            const ship = fleetRef.ships[viewShipIndex];
            if (!ship) return { current: 0, capacity: 0, remaining: 0 };
            const current = ship.cargo.reduce((s, r) => s + (r.quantity ?? 0), 0);
            // @ts-ignore get_cargo_capacity analog: front-end has no method; infer by size mapping
            const capacity = (() => {
                switch (ship.size) {
                    case 'Tiny': return 100;
                    case 'Small': return 250;
                    case 'Medium': return 500;
                    case 'Large': return 1000;
                    case 'Huge': return 2500;
                    case 'Planetary': return 5000;
                    default: return 0;
                }
            })();
            return { current, capacity, remaining: Math.max(0, capacity - current) };
        }
        // All ships
        const current = fleetRef.ships.reduce((s, ship) => s + ship.cargo.reduce((ss, r) => ss + (r.quantity ?? 0), 0), 0);
        const capacity = fleetRef.ships.reduce((s, ship) => s + (ship.size === 'Tiny' ? 100 : ship.size === 'Small' ? 250 : ship.size === 'Medium' ? 500 : ship.size === 'Large' ? 1000 : ship.size === 'Huge' ? 2500 : 5000), 0);
        return { current, capacity, remaining: Math.max(0, capacity - current) };
    };

    const renderTrade = () => {
        if (!selectedFleet) return <div>Select a fleet to view market.</div>;
        if (selectedFleet.current_system_id == null) return <div>Fleet is in deep space. Move to a planet to trade.</div>;
        if (!planetAtFleet) return <div>No co-located planet detected in this system.</div>;
        if (!market) return <div>Loading market...</div>;
        const credits = player ? player.credits.toLocaleString() : '—';
        const selected = selectedResource;
        const totalBuy = selected?.buy ? (selected.buy * qty).toFixed(3) : 'N/A';
        const totalSell = selected?.sell ? (selected.sell * qty).toFixed(3) : 'N/A';
        return (
            <div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
                    <h3>Trading at {planetAtFleet.name} (System #{system?.id})</h3>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                        <div>
                            <label style={{ marginRight: 6 }}>Ship:</label>
                            <select value={viewShipIndex} onChange={e => setViewShipIndex(Number(e.target.value))}>
                                <option value={-1}>All ships</option>
                                {selectedFleet.ships.map((s, idx) => (
                                    <option key={idx} value={idx}>{s.name || `Ship #${idx+1}`}</option>
                                ))}
                            </select>
                        </div>
                        <div style={{ background: 'rgba(0,255,0,0.1)', border: '1px solid rgba(0,255,0,0.3)', padding: '4px 8px', borderRadius: 4 }}>Credits: {credits} cr</div>
                    </div>
                </div>
                <div className="market-sections" style={{ display: 'grid', gridTemplateColumns: '3fr 2fr', gap: 20, alignItems: 'start' }}>
                    <div>
                        <table className="market-table">
                            <thead>
                                <tr>
                                    <th className="col-resource">Resource</th>
                                    <th className="col-available numeric">Available</th>
                                    <th className="col-buy numeric">Buy Price (cr)</th>
                                    <th className="col-sell numeric">Sell Price (cr)</th>
                                    <th className="col-stock numeric">Your Stock</th>
                                </tr>
                            </thead>
                            <tbody>
                                {market.resources.map((r: any, idx: number) => {
                                    const stocked = getStockForResource(r.resource_type);
                                    const isSel = selected?.resource_type === r.resource_type;
                                    return (
                                        <tr key={idx} className={isSel ? 'selected' : ''} onClick={() => setSelectedResource(r)}>
                                            <td className="col-resource">{r.resource_type}</td>
                                            <td className="col-available numeric">{r.quantity ?? 0}</td>
                                            <td className="col-buy numeric">{r.buy != null ? r.buy.toFixed(3) : 'N/A'}</td>
                                            <td className="col-sell numeric">{r.sell != null ? r.sell.toFixed(3) : 'N/A'}</td>
                                            <td className="col-stock numeric">{stocked}</td>
                                        </tr>
                                    );
                                })}
                            </tbody>
                        </table>
                    </div>
                    <div>
                        <div style={{ background: 'rgba(20,20,35,0.5)', border: '1px solid rgba(0,255,0,0.2)', padding: 12, borderRadius: 6 }}>
                            <h4>Trade Actions</h4>
                            {/* Capacity info */}
                            {selectedFleet && (
                                (() => {
                                    const cap = getCapacityInfo();
                                    return (
                                        <div style={{ marginBottom: 8, opacity: 0.9 }}>
                                            Cargo: {cap.current} / {cap.capacity} (remaining {cap.remaining})
                                        </div>
                                    );
                                })()
                            )}
                            <div style={{ marginBottom: 8 }}>Amount: <input type="number" min={1} value={qty} onChange={e => setQty(Math.max(1, Number(e.target.value)))} style={{ width: 90 }} /></div>
                            {selectedFleet && (
                                <div style={{ marginBottom: 8 }}>
                                    <label style={{ marginRight: 8 }}>Distribution:</label>
                                    <select value={distMode} onChange={e => setDistMode(e.target.value as any)}>
                                        <option value="first">First ship</option>
                                        <option value="even">Even across ships</option>
                                        <option value="selected">Selected ship</option>
                                    </select>
                                    {distMode === 'selected' && (
                                        <span style={{ marginLeft: 8, opacity: 0.9 }}>to: {viewShipIndex >= 0 ? (selectedFleet.ships[viewShipIndex]?.name || `Ship #${viewShipIndex+1}`) : 'choose a ship above'}</span>
                                    )}
                                </div>
                            )}
                            {/* No extra UI for selected ship; uses the top Ship selector. */}
                            <div style={{ display: 'flex', gap: 8 }}>
                                <button
                                    onClick={() => selected && handleBuy(selected.resource_type as unknown as ResourceType)}
                                    disabled={!selected || !selected.buy || (distMode === 'selected' && viewShipIndex < 0)}
                                >
                                    Buy ({totalBuy} cr)
                                </button>
                                <button
                                    onClick={() => selected && handleSell(selected.resource_type as unknown as ResourceType)}
                                    disabled={!selected || !selected.sell || (distMode === 'selected' && viewShipIndex < 0)}
                                >
                                    Sell ({totalSell} cr)
                                </button>
                            </div>
                            {!selected && <div style={{ marginTop: 8, opacity: 0.8 }}>Select a resource row first</div>}
                            {selected && distMode === 'selected' && viewShipIndex < 0 && (
                                <div style={{ marginTop: 6, opacity: 0.8 }}>Choose a ship from the Ship selector above.</div>
                            )}
                            {/* Ship market access moved to Ships tab */}
                        </div>
                    </div>
                </div>
            </div>
        );
    };

    // Removed in-view ship list; the Ships tab opens the modal instead

    const renderCompare = () => {
        if (!selectedFleet) return <div>Select a fleet first.</div>;
        const fleetPosGal = selectedFleet.position;
        // Build selected list with metadata
        const selectedEntries = compareSelected
            .map(k => {
                const [sidStr, pidStr] = k.split('-');
                const sid = Number(sidStr), pid = Number(pidStr);
                const sys = galaxy.find(s => s.id === sid);
                if (!sys) return null;
                const p = sys.planets[pid];
                if (!p) return null;
                const dist = computeTravelDistanceToPlanet(selectedFleet, sys, pid).toFixed(1);
                return { key: k, sys, planet: p, dist };
            })
            .filter(Boolean) as { key: string; sys: StarSystem; planet: any; dist: string }[];

        // Gather all resource types from selected markets
        const allResourceTypes: string[] = Array.from(new Set(selectedEntries.flatMap(entry => (compareMarkets[entry.key]?.resources || []).map((r: any) => r.resource_type))));
        allResourceTypes.sort();
        const headerRowSpan = selectedEntries.length > 0 ? 2 : 1;

        return (
            <div>
                <h3>Market Comparison</h3>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 2fr', gap: 16 }}>
                    <div>
                        <div style={{ marginBottom: 8 }}>
                            <input
                                type="text"
                                placeholder="Search system or planet..."
                                value={compareSearch}
                                onChange={e => setCompareSearch(e.target.value)}
                                style={{ width: '100%' }}
                            />
                        </div>
                        <table className="market-table" style={{ tableLayout: 'fixed', width: '100%' }}>
                            <thead>
                                <tr>
                                    <th>System</th>
                                    <th>Planet</th>
                                    <th className="numeric">Distance</th>
                                    <th>Select</th>
                                </tr>
                            </thead>
                            <tbody>
                                {galaxy.flatMap((sys) => sys.planets
                                    .map((p, idx) => ({ sys, p, idx }))
                                    .filter(({ sys, p }) => {
                                        const q = compareSearch.trim().toLowerCase();
                                        if (!q) return true;
                                        return sys.star.name.toLowerCase().includes(q) || p.name.toLowerCase().includes(q);
                                    })
                                    .map(({ sys, p, idx }) => {
                                    const dist = computeTravelDistanceToPlanet(selectedFleet, sys, idx).toFixed(1);
                                    const key = planetKey(sys.id, idx);
                                    const checked = isSelectedForCompare(sys.id, idx);
                                    return (
                                        <tr key={`${sys.id}-${idx}`} className={checked ? 'selected' : ''}>
                                            <td>{sys.star.name} (#{sys.id})</td>
                                            <td>{p.name}</td>
                                            <td className="numeric">{dist}</td>
                                            <td><input type="checkbox" checked={checked} onChange={() => toggleCompareSelect(sys.id, idx)} /></td>
                                        </tr>
                                    );
                                }))}
                            </tbody>
                        </table>
                    </div>
                    <div style={{ overflowX: 'auto', maxWidth: '100%' }}>
                        <table className="market-table" style={{ whiteSpace: 'nowrap', tableLayout: 'fixed' }}>
                            <thead>
                                <tr>
                                    <th rowSpan={headerRowSpan}>Resource</th>
                                    {selectedEntries.map(entry => (
                                        <th key={entry.key} colSpan={3}>{entry.planet.name} ({entry.dist})</th>
                                    ))}
                                    {selectedEntries.length > 0 && <th rowSpan={headerRowSpan}>Your Stock</th>}
                                </tr>
                                {selectedEntries.length > 0 && (
                                    <tr>
                                        {selectedEntries.map(entry => (
                                            <>
                                                <th key={entry.key+':a'} className="numeric">Avail</th>
                                                <th key={entry.key+':b'} className="numeric">Buy</th>
                                                <th key={entry.key+':c'} className="numeric">Sell</th>
                                            </>
                                        ))}
                                    </tr>
                                )}
                            </thead>
                            <tbody>
                                {selectedEntries.length === 0 && (
                                    <tr><td>Select planets on the left to compare.</td></tr>
                                )}
                                {selectedEntries.length > 0 && allResourceTypes.map((rt) => {
                                    // Determine best prices for highlight
                                    const prices = selectedEntries.map(e => {
                                        const res = (compareMarkets[e.key]?.resources || []).find((r: any) => r.resource_type === rt);
                                        return { buy: res?.buy, sell: res?.sell };
                                    });
                                    const buys = prices.map(p => p.buy).filter((v): v is number => typeof v === 'number');
                                    const sells = prices.map(p => p.sell).filter((v): v is number => typeof v === 'number');
                                    // Best to SELL: highest buy price; Best to BUY: lowest sell price
                                    const bestBuy = buys.length ? Math.max(...buys) : undefined;
                                    const bestSell = sells.length ? Math.min(...sells) : undefined;
                                    return (
                                        <tr key={rt}>
                                            <td>{rt}</td>
                                            {selectedEntries.map(e => {
                                                const res = (compareMarkets[e.key]?.resources || []).find((r: any) => r.resource_type === rt);
                                                const buy = res?.buy;
                                                const sell = res?.sell;
                                                const avail = res?.quantity ?? 0;
                                                return (
                                                    <>
                                                        <td key={e.key+rt+':q'} className="numeric">{avail}</td>
                                                        <td key={e.key+rt+':b'} className={`numeric ${buy!=null && buy===bestBuy ? 'best-buy' : ''}`}>{buy != null ? buy.toFixed(3) : 'N/A'}</td>
                                                        <td key={e.key+rt+':s'} className={`numeric ${sell!=null && sell===bestSell ? 'best-sell' : ''}`}>{sell != null ? sell.toFixed(3) : 'N/A'}</td>
                                                    </>
                                                );
                                            })}
                                            <td className="numeric">{getStockForResource(rt)}</td>
                                        </tr>
                                    );
                                })}
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        );
    };

    return (
        <div className="market-view">
            <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
                <button className={active==='trade' ? 'active' : ''} onClick={() => setActive('trade')}>Trade</button>
                <button onClick={() => setShowShipModal(true)}>Ships</button>
                <button className={active==='compare' ? 'active' : ''} onClick={() => setActive('compare')}>Compare</button>
            </div>
            {error && <div className="error">{error}</div>}
            {active === 'trade' && renderTrade()}
            {active === 'compare' && renderCompare()}
            {showShipModal && planetIndex != null && system && (
            <ShipMarketModal
                isOpen={showShipModal}
                onClose={() => setShowShipModal(false)}
                systemId={system?.id as number}
                planetId={planetIndex as number}
            />
            )}
        </div>
    );
};

export default MarketView;