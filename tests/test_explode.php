<?php
$s = "a_b_c_d_e_f_g_h_i_j_k_l_m_n_o_p";
for ($i = 0; $i < 200; $i++) {
    $p = explode("_", $s);
}
echo "OK";
