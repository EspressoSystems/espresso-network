use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct LeaderViewStats<V> {
    pub view: V,
    pub prev_proposal_send: Option<i128>,
    pub proposal_send: Option<i128>,
    pub vote_recv: Option<i128>,
    pub da_proposal_send: Option<i128>,
    pub builder_start: Option<i128>,
    pub block_built: Option<i128>,
    pub vid_disperse_send: Option<i128>,
    pub timeout_certificate_formed: Option<i128>,
    pub qc_formed: Option<i128>,
    pub da_cert_send: Option<i128>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ReplicaViewStats<V> {
    pub view: V,
    pub view_change: Option<i128>,
    pub proposal_timestamp: Option<i128>,
    pub proposal_recv: Option<i128>,
    pub vote_send: Option<i128>,
    pub timeout_vote_send: Option<i128>,
    pub da_proposal_received: Option<i128>,
    pub da_proposal_validated: Option<i128>,
    pub da_certificate_recv: Option<i128>,
    pub proposal_prelim_validated: Option<i128>,
    pub proposal_validated: Option<i128>,
    pub timeout_triggered: Option<i128>,
    pub vid_share_validated: Option<i128>,
    pub vid_share_recv: Option<i128>,
}

impl<V> LeaderViewStats<V> {
    pub fn new(view: V) -> Self {
        Self {
            view,
            prev_proposal_send: None,
            proposal_send: None,
            vote_recv: None,
            da_proposal_send: None,
            builder_start: None,
            block_built: None,
            vid_disperse_send: None,
            timeout_certificate_formed: None,
            qc_formed: None,
            da_cert_send: None,
        }
    }
}

impl<V> ReplicaViewStats<V> {
    pub fn new(view: V) -> Self {
        Self {
            view,
            view_change: None,
            proposal_timestamp: None,
            proposal_recv: None,
            vote_send: None,
            timeout_vote_send: None,
            da_proposal_received: None,
            da_proposal_validated: None,
            da_certificate_recv: None,
            proposal_prelim_validated: None,
            proposal_validated: None,
            timeout_triggered: None,
            vid_share_validated: None,
            vid_share_recv: None,
        }
    }
}
