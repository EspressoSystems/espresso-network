// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item "><a href="index.html">Home</a></li><li class="chapter-item affix "><li class="part-title">contracts</li><li class="chapter-item "><a href="contracts/src/index.html">❱ src</a><a class="toggle"><div>❱</div></a></li><li><ol class="section"><li class="chapter-item "><a href="contracts/src/interfaces/index.html">❱ interfaces</a><a class="toggle"><div>❱</div></a></li><li><ol class="section"><li class="chapter-item "><a href="contracts/src/interfaces/ILightClient.sol/interface.ILightClient.html">ILightClient</a></li><li class="chapter-item "><a href="contracts/src/interfaces/IPlonkVerifier.sol/interface.IPlonkVerifier.html">IPlonkVerifier</a></li><li class="chapter-item "><a href="contracts/src/interfaces/IRewardClaim.sol/interface.IRewardClaim.html">IRewardClaim</a></li></ol></li><li class="chapter-item "><a href="contracts/src/legacy/index.html">❱ legacy</a><a class="toggle"><div>❱</div></a></li><li><ol class="section"><li class="chapter-item "><a href="contracts/src/legacy/Transcript.sol/library.Transcript.html">Transcript</a></li></ol></li><li class="chapter-item "><a href="contracts/src/libraries/index.html">❱ libraries</a><a class="toggle"><div>❱</div></a></li><li><ol class="section"><li class="chapter-item "><a href="contracts/src/libraries/BLSSig.sol/library.BLSSig.html">BLSSig</a></li><li class="chapter-item "><a href="contracts/src/libraries/ERC1967Proxy.sol/contract.ERC1967Proxy.html">ERC1967Proxy</a></li><li class="chapter-item "><a href="contracts/src/libraries/EdOnBn254.sol/library.EdOnBN254.html">EdOnBN254</a></li><li class="chapter-item "><a href="contracts/src/libraries/LightClientStateUpdateVK.sol/library.LightClientStateUpdateVK.html">LightClientStateUpdateVK</a></li><li class="chapter-item "><a href="contracts/src/libraries/LightClientStateUpdateVKV2.sol/library.LightClientStateUpdateVKV2.html">LightClientStateUpdateVKV2</a></li><li class="chapter-item "><a href="contracts/src/libraries/LightClientStateUpdateVKV3.sol/library.LightClientStateUpdateVKV3.html">LightClientStateUpdateVKV3</a></li><li class="chapter-item "><a href="contracts/src/libraries/PlonkVerifier.sol/library.PlonkVerifier.html">PlonkVerifier</a></li><li class="chapter-item "><a href="contracts/src/libraries/PlonkVerifierV2.sol/library.PlonkVerifierV2.html">PlonkVerifierV2</a></li><li class="chapter-item "><a href="contracts/src/libraries/PlonkVerifierV3.sol/library.PlonkVerifierV3.html">PlonkVerifierV3</a></li><li class="chapter-item "><a href="contracts/src/libraries/PolynomialEval.sol/library.PolynomialEval.html">PolynomialEval</a></li><li class="chapter-item "><a href="contracts/src/libraries/PolynomialEvalV2.sol/library.PolynomialEvalV2.html">PolynomialEvalV2</a></li><li class="chapter-item "><a href="contracts/src/libraries/PolynomialEvalV3.sol/library.PolynomialEvalV3.html">PolynomialEvalV3</a></li><li class="chapter-item "><a href="contracts/src/libraries/RewardMerkleTreeVerifier.sol/library.RewardMerkleTreeVerifier.html">RewardMerkleTreeVerifier</a></li></ol></li><li class="chapter-item "><a href="contracts/src/EspToken.sol/contract.EspToken.html">EspToken</a></li><li class="chapter-item "><a href="contracts/src/EspTokenV2.sol/contract.EspTokenV2.html">EspTokenV2</a></li><li class="chapter-item "><a href="contracts/src/FeeContract.sol/contract.FeeContract.html">FeeContract</a></li><li class="chapter-item "><a href="contracts/src/InitializedAt.sol/contract.InitializedAt.html">InitializedAt</a></li><li class="chapter-item "><a href="contracts/src/LightClient.sol/contract.LightClient.html">LightClient</a></li><li class="chapter-item "><a href="contracts/src/LightClientArbitrum.sol/interface.ArbSys.html">ArbSys</a></li><li class="chapter-item "><a href="contracts/src/LightClientArbitrum.sol/contract.LightClientArbitrum.html">LightClientArbitrum</a></li><li class="chapter-item "><a href="contracts/src/LightClientArbitrumV2.sol/interface.ArbSys.html">ArbSys</a></li><li class="chapter-item "><a href="contracts/src/LightClientArbitrumV2.sol/contract.LightClientArbitrumV2.html">LightClientArbitrumV2</a></li><li class="chapter-item "><a href="contracts/src/LightClientArbitrumV3.sol/interface.ArbSys.html">ArbSys</a></li><li class="chapter-item "><a href="contracts/src/LightClientArbitrumV3.sol/contract.LightClientArbitrumV3.html">LightClientArbitrumV3</a></li><li class="chapter-item "><a href="contracts/src/LightClientV2.sol/contract.LightClientV2.html">LightClientV2</a></li><li class="chapter-item "><a href="contracts/src/LightClientV3.sol/contract.LightClientV3.html">LightClientV3</a></li><li class="chapter-item "><a href="contracts/src/OpsTimelock.sol/contract.OpsTimelock.html">OpsTimelock</a></li><li class="chapter-item "><a href="contracts/src/RewardClaim.sol/contract.RewardClaim.html">RewardClaim</a></li><li class="chapter-item "><a href="contracts/src/SafeExitTimelock.sol/contract.SafeExitTimelock.html">SafeExitTimelock</a></li><li class="chapter-item "><a href="contracts/src/StakeTable.sol/contract.StakeTable.html">StakeTable</a></li><li class="chapter-item "><a href="contracts/src/StakeTableV2.sol/contract.StakeTableV2.html">StakeTableV2</a></li></ol></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
