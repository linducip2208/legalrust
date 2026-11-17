# EFFORT ROLLUP (computed from catalog complexity; planning estimate, NOT a commitment)
Assumptions: midpoint person-days S=4, M=7.5, L=20, XL=40; 20 working-days/month; excludes counsel wait, benchmarks, PM/QA overhead (+35% loaded est.).

Mix: {'XL': 263, 'L': 252, 'M': 261, 'S': 224}. Raw total: 18,414 person-days = 921 person-months (~76.7 person-years unloaded; ~1,243 PM loaded).

| Domain | Features | Person-days (mid) | Person-months |
|---|---|---|---|
| D01 | 40 | 895 | 45 |
| D02 | 40 | 794 | 40 |
| D03 | 40 | 660 | 33 |
| D04 | 40 | 604 | 30 |
| D05 | 40 | 808 | 40 |
| D06 | 40 | 857 | 43 |
| D07 | 40 | 876 | 44 |
| D08 | 40 | 602 | 30 |
| D09 | 40 | 660 | 33 |
| D10 | 40 | 782 | 39 |
| D11 | 40 | 718 | 36 |
| D12 | 40 | 826 | 41 |
| D13 | 40 | 706 | 35 |
| D14 | 40 | 730 | 36 |
| D15 | 40 | 694 | 35 |
| D16 | 40 | 646 | 32 |
| D17 | 40 | 791 | 40 |
| D18 | 40 | 760 | 38 |
| D19 | 40 | 720 | 36 |
| D20 | 40 | 775 | 39 |
| D21 | 40 | 587 | 29 |
| D22 | 40 | 728 | 36 |
| D23 | 40 | 699 | 35 |
| D24 | 40 | 769 | 38 |
| D25 | 40 | 727 | 36 |

Scenarios (loaded, dev-only): 6 devs → ~207 mo; 10 devs → ~124 mo; 15 devs → ~83 mo (excludes B-001..B-004 wait + phased scope; P0-first per roadmap).
Critical path (must sequence): D16/D17 → D02 → D03 → D04 → D05 → D06 → D09/D10 → D15/D14. Non-path domains parallelize (max 4-8 teams).
