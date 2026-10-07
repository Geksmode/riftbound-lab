from final_experiments import *
N=4000
for name,d in (("Stock",AKALI_STOCK),("C5",C5)):
    for bf in ("Void Gate","Targon's Peak"):
        for mull in ("default","aggro"):
            wr,_,_=play(N,d,LEBLANC,bf,LB_BFS,dict(cfg,mulligan=mull),seed0=5*10**6)
            wr2,_,_=play(N,d,LEBLANC_SIDED,bf,LB_BFS,dict(cfg,mulligan=mull),seed0=6*10**6)
            print(f"{name:6s} {bf:14s} mull={mull:7s} vsLB {wr:.1%} vsLBside {wr2:.1%}",flush=True)
