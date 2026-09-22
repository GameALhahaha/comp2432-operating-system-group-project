import pandas as pd
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import matplotlib.style as mplstyle
import numpy as np

mplstyle.use('seaborn-v0_8-whitegrid')
plt.rcParams['font.family'] = 'Noto Sans CJK SC'
plt.rcParams['font.size'] = 12
plt.rcParams['axes.titlesize'] = 15
plt.rcParams['axes.labelsize'] = 13

df = pd.read_csv('/home/ubuntu/benchmark_results.csv') # Change this file path of the your csv file location

best = df.loc[df.groupby(['robots', 'tasks'])['throughput_tasks_per_s'].idxmax()].copy()
best = best.sort_values(['robots', 'tasks'])

colors = {1: '#2196F3', 2: '#4CAF50', 4: '#FF9800', 8: '#E91E63', 16: '#9C27B0'}

# ========== Chart 1: Throughput vs Task Count (by Robot Count) ==========
fig, ax = plt.subplots(figsize=(12, 7))
for rc in sorted(best['robots'].unique()):
    subset = best[best['robots'] == rc]
    ax.plot(subset['tasks'], subset['throughput_tasks_per_s'],
            marker='o', linewidth=2.5, markersize=8,
            label=f'{rc} robots', color=colors[rc])

ax.set_xlabel('Task Count')
ax.set_ylabel('Throughput (tasks/s)')
ax.set_title('Throughput vs Task Count (by Robot Count)')
ax.set_xscale('log')
ax.legend(title='Robot Count', fontsize=11, title_fontsize=12)
ax.yaxis.set_major_formatter(plt.FuncFormatter(lambda x, _: f'{x:,.0f}'))
plt.tight_layout()
fig.savefig('/home/ubuntu/chart1_throughput_vs_tasks.png', dpi=150)  # Change this file path of the your csv file location
plt.close()

# ========== Chart 2: Throughput vs Robot Count (by Task Count) ==========
fig, ax = plt.subplots(figsize=(12, 7))
task_colors = {1000: '#2196F3', 5000: '#4CAF50', 10000: '#FF9800',
               20000: '#E91E63', 50000: '#9C27B0', 100000: '#795548'}
for tc in sorted(best['tasks'].unique()):
    subset = best[best['tasks'] == tc]
    ax.plot(subset['robots'], subset['throughput_tasks_per_s'],
            marker='s', linewidth=2.5, markersize=8,
            label=f'{tc:,} tasks', color=task_colors[tc])

ax.set_xlabel('Robot Count')
ax.set_ylabel('Throughput (tasks/s)')
ax.set_title('Throughput vs Robot Count (by Task Count)')
ax.set_xticks([1, 2, 4, 8, 16])
ax.legend(title='Task Count', fontsize=10, title_fontsize=11)
ax.yaxis.set_major_formatter(plt.FuncFormatter(lambda x, _: f'{x:,.0f}'))
plt.tight_layout()
fig.savefig('/home/ubuntu/chart2_throughput_vs_robots.png', dpi=150)  # Change this file path of the your csv file location
plt.close()

# ========== Chart 3: Elapsed Time vs Task Count (by Robot Count) ==========
fig, ax = plt.subplots(figsize=(12, 7))
for rc in sorted(best['robots'].unique()):
    subset = best[best['robots'] == rc]
    ax.plot(subset['tasks'], subset['elapsed_s'] * 1000,
            marker='^', linewidth=2.5, markersize=8,
            label=f'{rc} robots', color=colors[rc])

ax.set_xlabel('Task Count')
ax.set_ylabel('Elapsed Time (ms)')
ax.set_title('Elapsed Time vs Task Count (by Robot Count)')
ax.set_xscale('log')
ax.set_yscale('log')
ax.legend(title='Robot Count', fontsize=11, title_fontsize=12)
plt.tight_layout()
fig.savefig('/home/ubuntu/chart3_elapsed_vs_tasks.png', dpi=150)  # Change this file path of the your csv file location
plt.close()

# ========== Chart 4: Heatmap of Throughput ==========
pivot = best.pivot_table(index='robots', columns='tasks', values='throughput_tasks_per_s')
fig, ax = plt.subplots(figsize=(12, 6))
im = ax.imshow(pivot.values, cmap='YlOrRd', aspect='auto')
ax.set_xticks(range(len(pivot.columns)))
ax.set_xticklabels([f'{c:,}' for c in pivot.columns])
ax.set_yticks(range(len(pivot.index)))
ax.set_yticklabels(pivot.index)
ax.set_xlabel('Task Count')
ax.set_ylabel('Robot Count')
ax.set_title('Throughput Heatmap (tasks/s)')

for i in range(len(pivot.index)):
    for j in range(len(pivot.columns)):
        val = pivot.values[i, j]
        text_color = 'white' if val > pivot.values.max() * 0.6 else 'black'
        ax.text(j, i, f'{val:,.0f}', ha='center', va='center',
                fontsize=9, color=text_color, fontweight='bold')

cbar = plt.colorbar(im, ax=ax)
cbar.ax.yaxis.set_major_formatter(plt.FuncFormatter(lambda x, _: f'{x:,.0f}'))
plt.tight_layout()
fig.savefig('/home/ubuntu/chart4_heatmap.png', dpi=150)  # Change this file path of the your csv file location
plt.close() 

# ========== Summary Table CSV ==========
summary = best[['robots', 'tasks', 'completed', 'elapsed_s', 'throughput_tasks_per_s']].copy()
summary.columns = ['Robots', 'Tasks', 'Completed', 'Elapsed (s)', 'Throughput (tasks/s)']
summary.to_csv('/home/ubuntu/benchmark_summary.csv', index=False)  # Change this file path of the your csv file location

print("All charts and summary generated successfully.")
