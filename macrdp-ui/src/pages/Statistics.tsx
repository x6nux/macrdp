import { useState, useEffect, useMemo } from "react";
import { BarChart3, Network, ChevronLeft, ChevronRight } from "lucide-react";
import { api } from "../lib/ipc";
import { formatBytes, formatDuration } from "../lib/format";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import type { ConnectionHistory, TrafficStats } from "../lib/types";

const PAGE_SIZE = 20;

function Statistics() {
  // Connection history
  const [history, setHistory] = useState<ConnectionHistory[]>([]);
  const [page, setPage] = useState(0);
  const [hasMore, setHasMore] = useState(true);

  // Traffic stats
  const [trafficStats, setTrafficStats] = useState<TrafficStats[]>([]);

  useEffect(() => {
    fetchHistory(0);
    api
      .getTrafficStats(30)
      .then(setTrafficStats)
      .catch(console.error);
  }, []);

  const fetchHistory = async (p: number) => {
    try {
      const data = await api.getConnectionHistory(PAGE_SIZE, p * PAGE_SIZE);
      setHistory(data);
      setPage(p);
      setHasMore(data.length === PAGE_SIZE);
    } catch (err) {
      console.error("Failed to fetch connection history:", err);
    }
  };

  const totalTraffic = useMemo(
    () => trafficStats.reduce((sum, d) => sum + d.bytes_sent, 0),
    [trafficStats],
  );

  const totalConnections = useMemo(
    () => trafficStats.reduce((sum, d) => sum + d.connection_count, 0),
    [trafficStats],
  );

  const maxBytes = useMemo(
    () => Math.max(...trafficStats.map((d) => d.bytes_sent), 1),
    [trafficStats],
  );

  const formatDate = (dateStr: string) => {
    try {
      const d = new Date(dateStr);
      return d.toLocaleDateString("zh-CN", {
        month: "2-digit",
        day: "2-digit",
      });
    } catch {
      return dateStr;
    }
  };

  const formatDateTime = (dateStr: string) => {
    try {
      const d = new Date(dateStr);
      return d.toLocaleString("zh-CN", {
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      });
    } catch {
      return dateStr;
    }
  };

  return (
    <div className="space-y-6">
      {/* Traffic stats section */}
      <section>
        <h2 className="mb-3 text-base font-medium text-foreground">
          流量统计
        </h2>

        {/* Summary */}
        <div className="mb-4 grid grid-cols-2 gap-4">
          <Card size="sm">
            <CardContent className="flex items-center gap-3">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-muted">
                <BarChart3 className="h-4 w-4 text-muted-foreground" />
              </div>
              <div>
                <div className="text-xs text-muted-foreground">30 天总流量</div>
                <div className="mt-0.5 text-lg font-semibold text-foreground">
                  {formatBytes(totalTraffic)}
                </div>
              </div>
            </CardContent>
          </Card>
          <Card size="sm">
            <CardContent className="flex items-center gap-3">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-muted">
                <Network className="h-4 w-4 text-muted-foreground" />
              </div>
              <div>
                <div className="text-xs text-muted-foreground">30 天总连接数</div>
                <div className="mt-0.5 text-lg font-semibold text-foreground">
                  {totalConnections}
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Bar chart */}
        {trafficStats.length === 0 ? (
          <Card size="sm">
            <CardContent className="py-8 text-center text-sm text-muted-foreground">
              暂无流量数据
            </CardContent>
          </Card>
        ) : (
          <Card size="sm">
            <CardContent>
              <div className="space-y-1.5">
                {trafficStats.map((day) => (
                  <div key={day.date} className="flex items-center gap-3">
                    <span className="w-12 flex-shrink-0 text-right font-mono text-xs text-muted-foreground">
                      {formatDate(day.date)}
                    </span>
                    <div className="flex-1">
                      <div
                        className="h-5 rounded bg-primary/60 transition-all"
                        style={{
                          width: `${Math.max((day.bytes_sent / maxBytes) * 100, 1)}%`,
                        }}
                      />
                    </div>
                    <span className="w-20 flex-shrink-0 text-right font-mono text-xs text-muted-foreground">
                      {formatBytes(day.bytes_sent)}
                    </span>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        )}
      </section>

      {/* Connection history table */}
      <section>
        <h2 className="mb-3 text-base font-medium text-foreground">
          连接历史
        </h2>

        {history.length === 0 && page === 0 ? (
          <Card size="sm">
            <CardContent className="py-8 text-center text-sm text-muted-foreground">
              暂无连接记录
            </CardContent>
          </Card>
        ) : (
          <>
            <div className="overflow-hidden rounded-lg border border-border">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-border bg-muted/50">
                    <th className="px-3 py-2 text-left text-xs font-medium text-muted-foreground">
                      客户端
                    </th>
                    <th className="px-3 py-2 text-left text-xs font-medium text-muted-foreground">
                      IP
                    </th>
                    <th className="px-3 py-2 text-left text-xs font-medium text-muted-foreground">
                      连接时间
                    </th>
                    <th className="px-3 py-2 text-left text-xs font-medium text-muted-foreground">
                      断开时间
                    </th>
                    <th className="px-3 py-2 text-right text-xs font-medium text-muted-foreground">
                      时长
                    </th>
                    <th className="px-3 py-2 text-right text-xs font-medium text-muted-foreground">
                      流量
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border/50">
                  {history.map((conn) => (
                    <tr
                      key={conn.id}
                      className="bg-card transition-colors hover:bg-muted/30"
                    >
                      <td className="px-3 py-2 text-foreground">
                        {conn.client_name || "-"}
                      </td>
                      <td className="px-3 py-2 font-mono text-xs text-muted-foreground">
                        {conn.client_ip}
                      </td>
                      <td className="px-3 py-2 text-xs text-muted-foreground">
                        {formatDateTime(conn.connected_at)}
                      </td>
                      <td className="px-3 py-2 text-xs text-muted-foreground">
                        {formatDateTime(conn.disconnected_at)}
                      </td>
                      <td className="px-3 py-2 text-right text-xs text-muted-foreground">
                        {formatDuration(conn.duration_secs)}
                      </td>
                      <td className="px-3 py-2 text-right text-xs text-muted-foreground">
                        {formatBytes(conn.bytes_total)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {/* Pagination */}
            <div className="mt-3 flex items-center justify-between">
              <span className="text-xs text-muted-foreground">
                第 {page + 1} 页
              </span>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  disabled={page === 0}
                  onClick={() => fetchHistory(page - 1)}
                >
                  <ChevronLeft className="h-3.5 w-3.5" />
                  上一页
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={!hasMore}
                  onClick={() => fetchHistory(page + 1)}
                >
                  下一页
                  <ChevronRight className="h-3.5 w-3.5" />
                </Button>
              </div>
            </div>
          </>
        )}
      </section>
    </div>
  );
}

export default Statistics;
