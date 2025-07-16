use std::time::{Duration, Instant};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration: Duration,
    pub operations_per_second: f64,
    pub memory_usage: Option<usize>,
    pub success: bool,
    pub error: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    pub name: String,
    pub results: Vec<BenchmarkResult>,
    pub total_duration: Duration,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct PerformanceBenchmarks {
    results: Vec<BenchmarkResult>,
}

impl PerformanceBenchmarks {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    pub async fn run_all_benchmarks(&mut self) -> BenchmarkSuite {
        let start_time = Instant::now();
        
        // Terminal rendering benchmarks
        self.benchmark_terminal_rendering().await;
        self.benchmark_command_execution().await;
        self.benchmark_fuzzy_matching().await;
        self.benchmark_ai_processing().await;
        self.benchmark_workflow_execution().await;
        self.benchmark_plugin_loading().await;
        self.benchmark_memory_usage().await;
        
        let total_duration = start_time.elapsed();
        
        BenchmarkSuite {
            name: "NeoPilot Terminal Performance Suite".to_string(),
            results: self.results.clone(),
            total_duration,
            timestamp: chrono::Utc::now(),
        }
    }

    async fn benchmark_terminal_rendering(&mut self) {
        let start = Instant::now();
        let mut operations = 0;
        
        // Simulate terminal rendering operations
        for _ in 0..1000 {
            // Simulate rendering a block with 100 lines
            let _rendered_content = self.simulate_block_rendering(100);
            operations += 1;
        }
        
        let duration = start.elapsed();
        let ops_per_sec = operations as f64 / duration.as_secs_f64();
        
        self.results.push(BenchmarkResult {
            name: "Terminal Rendering".to_string(),
            duration,
            operations_per_second: ops_per_sec,
            memory_usage: None,
            success: true,
            error: None,
            metadata: {
                let mut map = HashMap::new();
                map.insert("blocks_rendered".to_string(), serde_json::Value::Number(operations.into()));
                map.insert("lines_per_block".to_string(), serde_json::Value::Number(100.into()));
                map
            },
        });
    }

    async fn benchmark_command_execution(&mut self) {
        let start = Instant::now();
        let mut successful_commands = 0;
        let mut failed_commands = 0;
        
        // Simulate command execution
        for i in 0..100 {
            let command_start = Instant::now();
            
            // Simulate different command types
            let success = match i % 4 {
                0 => self.simulate_fast_command().await,
                1 => self.simulate_medium_command().await,
                2 => self.simulate_slow_command().await,
                _ => self.simulate_failing_command().await,
            };
            
            if success {
                successful_commands += 1;
            } else {
                failed_commands += 1;
            }
        }
        
        let duration = start.elapsed();
        let ops_per_sec = (successful_commands + failed_commands) as f64 / duration.as_secs_f64();
        
        self.results.push(BenchmarkResult {
            name: "Command Execution".to_string(),
            duration,
            operations_per_second: ops_per_sec,
            memory_usage: None,
            success: failed_commands == 0,
            error: if failed_commands > 0 { 
                Some(format!("{} commands failed", failed_commands)) 
            } else { 
                None 
            },
            metadata: {
                let mut map = HashMap::new();
                map.insert("successful_commands".to_string(), serde_json::Value::Number(successful_commands.into()));
                map.insert("failed_commands".to_string(), serde_json::Value::Number(failed_commands.into()));
                map
            },
        });
    }

    async fn benchmark_fuzzy_matching(&mut self) {
        let start = Instant::now();
        let test_strings = self.generate_test_strings(10000);
        let queries = vec!["test", "file", "command", "workflow", "plugin"];
        
        let mut total_matches = 0;
        
        for query in &queries {
            for test_string in &test_strings {
                if crate::fuzzy_match::fuzzy_match(query, test_string) > 0.5 {
                    total_matches += 1;
                }
            }
        }
        
        let duration = start.elapsed();
        let ops_per_sec = (test_strings.len() * queries.len()) as f64 / duration.as_secs_f64();
        
        self.results.push(BenchmarkResult {
            name: "Fuzzy Matching".to_string(),
            duration,
            operations_per_second: ops_per_sec,
            memory_usage: None,
            success: true,
            error: None,
            metadata: {
                let mut map = HashMap::new();
                map.insert("test_strings".to_string(), serde_json::Value::Number(test_strings.len().into()));
                map.insert("queries".to_string(), serde_json::Value::Number(queries.len().into()));
                map.insert("total_matches".to_string(), serde_json::Value::Number(total_matches.into()));
                map
            },
        });
    }

    async fn benchmark_ai_processing(&mut self) {
        let start = Instant::now();
        let mut processed_messages = 0;
        
        // Simulate AI message processing
        for _ in 0..50 {
            let message_start = Instant::now();
            
            // Simulate AI processing time
            sleep(Duration::from_millis(10)).await;
            
            processed_messages += 1;
        }
        
        let duration = start.elapsed();
        let ops_per_sec = processed_messages as f64 / duration.as_secs_f64();
        
        self.results.push(BenchmarkResult {
            name: "AI Processing".to_string(),
            duration,
            operations_per_second: ops_per_sec,
            memory_usage: None,
            success: true,
            error: None,
            metadata: {
                let mut map = HashMap::new();
                map.insert("messages_processed".to_string(), serde_json::Value::Number(processed_messages.into()));
                map
            },
        });
    }

    async fn benchmark_workflow_execution(&mut self) {
        let start = Instant::now();
        let mut workflows_executed = 0;
        
        // Simulate workflow execution
        for _ in 0..20 {
            // Simulate workflow with 5 steps
            for _ in 0..5 {
                sleep(Duration::from_millis(5)).await;
            }
            workflows_executed += 1;
        }
        
        let duration = start.elapsed();
        let ops_per_sec = workflows_executed as f64 / duration.as_secs_f64();
        
        self.results.push(BenchmarkResult {
            name: "Workflow Execution".to_string(),
            duration,
            operations_per_second: ops_per_sec,
            memory_usage: None,
            success: true,
            error: None,
            metadata: {
                let mut map = HashMap::new();
                map.insert("workflows_executed".to_string(), serde_json::Value::Number(workflows_executed.into()));
                map.insert("steps_per_workflow".to_string(), serde_json::Value::Number(5.into()));
                map
            },
        });
    }

    async fn benchmark_plugin_loading(&mut self) {
        let start = Instant::now();
        let mut plugins_loaded = 0;
        
        // Simulate plugin loading
        for _ in 0..10 {
            // Simulate plugin initialization time
            sleep(Duration::from_millis(20)).await;
            plugins_loaded += 1;
        }
        
        let duration = start.elapsed();
        let ops_per_sec = plugins_loaded as f64 / duration.as_secs_f64();
        
        self.results.push(BenchmarkResult {
            name: "Plugin Loading".to_string(),
            duration,
            operations_per_second: ops_per_sec,
            memory_usage: None,
            success: true,
            error: None,
            metadata: {
                let mut map = HashMap::new();
                map.insert("plugins_loaded".to_string(), serde_json::Value::Number(plugins_loaded.into()));
                map
            },
        });
    }

    async fn benchmark_memory_usage(&mut self) {
        let start = Instant::now();
        
        // Simulate memory-intensive operations
        let mut data_structures = Vec::new();
        
        for i in 0..1000 {
            let data = vec![i; 1000]; // 1000 integers
            data_structures.push(data);
        }
        
        let duration = start.elapsed();
        let memory_estimate = data_structures.len() * 1000 * std::mem::size_of::<i32>();
        
        // Clean up
        drop(data_structures);
        
        self.results.push(BenchmarkResult {
            name: "Memory Usage".to_string(),
            duration,
            operations_per_second: 1000.0 / duration.as_secs_f64(),
            memory_usage: Some(memory_estimate),
            success: true,
            error: None,
            metadata: {
                let mut map = HashMap::new();
                map.insert("allocated_structures".to_string(), serde_json::Value::Number(1000.into()));
                map.insert("estimated_memory_bytes".to_string(), serde_json::Value::Number(memory_estimate.into()));
                map
            },
        });
    }

    fn simulate_block_rendering(&self, lines: usize) -> String {
        // Simulate rendering by creating a string with the specified number of lines
        (0..lines)
            .map(|i| format!("Line {} with some content that needs to be rendered", i))
            .collect::<Vec<_>>()
            .join("\n")
    }

    async fn simulate_fast_command(&self) -> bool {
        sleep(Duration::from_millis(1)).await;
        true
    }

    async fn simulate_medium_command(&self) -> bool {
        sleep(Duration::from_millis(10)).await;
        true
    }

    async fn simulate_slow_command(&self) -> bool {
        sleep(Duration::from_millis(100)).await;
        true
    }

    async fn simulate_failing_command(&self) -> bool {
        sleep(Duration::from_millis(5)).await;
        false
    }

    fn generate_test_strings(&self, count: usize) -> Vec<String> {
        let prefixes = vec!["test", "file", "command", "workflow", "plugin", "config", "theme"];
        let suffixes = vec!["data", "info", "manager", "handler", "processor", "controller"];
        
        (0..count)
            .map(|i| {
                let prefix = &prefixes[i % prefixes.len()];
                let suffix = &suffixes[i % suffixes.len()];
                format!("{}_{}_{}_{}", prefix, suffix, i, i * 2)
            })
            .collect()
    }

    pub fn get_performance_summary(&self) -> String {
        if self.results.is_empty() {
            return "No benchmark results available".to_string();
        }

        let total_duration: Duration = self.results.iter().map(|r| r.duration).sum();
        let avg_ops_per_sec: f64 = self.results.iter().map(|r| r.operations_per_second).sum::<f64>() / self.results.len() as f64;
        let success_rate = self.results.iter().filter(|r| r.success).count() as f64 / self.results.len() as f64 * 100.0;

        // Find fastest and slowest based on operations_per_second
        let fastest_benchmark = self.results.iter()
            .max_by(|a, b| a.operations_per_second.partial_cmp(&b.operations_per_second).unwrap_or(std::cmp::Ordering::Equal));
        let slowest_benchmark = self.results.iter()
            .min_by(|a, b| a.operations_per_second.partial_cmp(&b.operations_per_second).unwrap_or(std::cmp::Ordering::Equal));

        format!(
            "Performance Summary:\n\
            - Total benchmarks: {}\n\
            - Total duration: {:.2}s\n\
            - Average ops/sec: {:.2}\n\
            - Success rate: {:.1}%\n\
            - Fastest benchmark: {} ({:.2} ops/sec)\n\
            - Slowest benchmark: {} ({:.2} ops/sec)",
            self.results.len(),
            total_duration.as_secs_f64(),
            avg_ops_per_sec,
            success_rate,
            fastest_benchmark.map(|r| &r.name).unwrap_or("N/A"),
            fastest_benchmark.map(|r| r.operations_per_second).unwrap_or(0.0),
            slowest_benchmark.map(|r| &r.name).unwrap_or("N/A"),
            slowest_benchmark.map(|r| r.operations_per_second).unwrap_or(f64::INFINITY)
        )
    }

    pub fn export_results(&self, format: &str) -> Result<String, Box<dyn std::error::Error>> {
        match format.to_lowercase().as_str() {
            "json" => Ok(serde_json::to_string_pretty(&self.results)?),
            "csv" => {
                let mut csv = String::from("name,duration_ms,ops_per_sec,success,error\n");
                for result in &self.results {
                    csv.push_str(&format!(
                        "{},{},{},{},{}\n",
                        result.name,
                        result.duration.as_millis(),
                        result.operations_per_second,
                        result.success,
                        result.error.as_deref().unwrap_or("")
                    ));
                }
                Ok(csv)
            }
            _ => Err("Unsupported format. Use 'json' or 'csv'".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_benchmark_creation() {
        let benchmarks = PerformanceBenchmarks::new();
        assert_eq!(benchmarks.results.len(), 0);
    }

    #[tokio::test]
    async fn test_terminal_rendering_benchmark() {
        let mut benchmarks = PerformanceBenchmarks::new();
        benchmarks.benchmark_terminal_rendering().await;
        
        assert_eq!(benchmarks.results.len(), 1);
        assert_eq!(benchmarks.results[0].name, "Terminal Rendering");
        assert!(benchmarks.results[0].success);
    }

    #[test]
    fn test_export_results() {
        let mut benchmarks = PerformanceBenchmarks::new();
        benchmarks.results.push(BenchmarkResult {
            name: "Test".to_string(),
            duration: Duration::from_millis(100),
            operations_per_second: 10.0,
            memory_usage: None,
            success: true,
            error: None,
            metadata: HashMap::new(),
        });

        let json_export = benchmarks.export_results("json").unwrap();
        assert!(json_export.contains("Test"));
        
        let csv_export = benchmarks.export_results("csv").unwrap();
        assert!(csv_export.contains("name,duration_ms"));
    }
}
