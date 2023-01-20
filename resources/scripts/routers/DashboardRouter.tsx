import React from 'react';
import tw from 'twin.macro';
import * as Icon from 'react-feather';
import { useStoreState } from 'easy-peasy';
import { useLocation } from 'react-router';
import TransitionRouter from '@/TransitionRouter';
import Spinner from '@/components/elements/Spinner';
import SidePanel from '@/components/elements/SidePanel';
import { NavLink, Route, Switch } from 'react-router-dom';
import { NotFound } from '@/components/elements/ScreenBlock';
import SubNavigation from '@/components/elements/SubNavigation';
import useWindowDimensions from '@/plugins/useWindowDimensions';
import MobileNavigation from '@/components/elements/MobileNavigation';
import ReferralContainer from '@/components/dashboard/ReferralContainer';
import DashboardContainer from '@/components/dashboard/DashboardContainer';
import AccountApiContainer from '@/components/dashboard/AccountApiContainer';
import InformationContainer from '@/components/elements/InformationContainer';
import AccountSSHContainer from '@/components/dashboard/ssh/AccountSSHContainer';
import AccountOverviewContainer from '@/components/dashboard/AccountOverviewContainer';
import AccountSecurityContainer from '@/components/dashboard/AccountSecurityContainer';
import CouponContainer from '@/components/dashboard/CouponContainer';

export default () => {
    const location = useLocation();
    const { width } = useWindowDimensions();
    const coupons = useStoreState((state) => state.settings.data!.coupons);
    const referrals = useStoreState((state) => state.storefront.data!.referrals.enabled);

    return (
        <>
            {width >= 1280 ? <SidePanel /> : <MobileNavigation />}
            {location.pathname.startsWith('/account') ? (
                <SubNavigation className={'j-down'}>
                    <div>
                        <NavLink to={'/account'} exact>
                            <div css={tw`flex items-center justify-between`}>
                                Contas <Icon.User css={tw`ml-1`} size={18} />
                            </div>
                        </NavLink>
                        <NavLink to={'/account/security'}>
                            <div css={tw`flex items-center justify-between`}>
                                Segurança <Icon.Key css={tw`ml-1`} size={18} />
                            </div>
                        </NavLink>
                        {referrals && (
                            <NavLink to={'/account/referrals'}>
                                <div css={tw`flex items-center justify-between`}>
                                    Referências <Icon.DollarSign css={tw`ml-1`} size={18} />
                                </div>
                            </NavLink>
                        )}
                        <NavLink to={'/account/api'}>
                            <div css={tw`flex items-center justify-between`}>
                                API <Icon.Code css={tw`ml-1`} size={18} />
                            </div>
                        </NavLink>
                        <NavLink to={'/account/ssh'}>
                            <div css={tw`flex items-center justify-between`}>
                                Chaves SSH <Icon.Terminal css={tw`ml-1`} size={18} />
                            </div>
                        </NavLink>
                        {coupons && (
                            <NavLink to={'/account/coupons'}>
                                <div className={'flex items-center justify-between'}>
                                    Cupons <Icon.DollarSign className={'ml-1'} size={18} />
                                </div>
                            </NavLink>
                        )}
                    </div>
                </SubNavigation>
            ) : (
                <>
                    {width >= 1024 && (
                        <SubNavigation className={'j-down lg:visible invisible'}>
                            <div>
                                <InformationContainer />
                            </div>
                        </SubNavigation>
                    )}
                </>
            )}
            <TransitionRouter>
                <React.Suspense fallback={<Spinner centered />}>
                    <Switch location={location}>
                        <Route path={'/'} exact>
                            <DashboardContainer />
                        </Route>
                        <Route path={'/account'} exact>
                            <AccountOverviewContainer />
                        </Route>
                        <Route path={'/account/security'} exact>
                            <AccountSecurityContainer />
                        </Route>
                        {referrals && (
                            <Route path={`/account/referrals`} exact>
                                <ReferralContainer />
                            </Route>
                        )}
                        <Route path={'/account/api'} exact>
                            <AccountApiContainer />
                        </Route>
                        <Route path={'/account/ssh'} exact>
                            <AccountSSHContainer />
                        </Route>
                        {coupons && (
                            <Route path={'/account/coupons'} exact>
                                <CouponContainer />
                            </Route>
                        )}
                        <Route path={'*'}>
                            <NotFound />
                        </Route>
                    </Switch>
                </React.Suspense>
            </TransitionRouter>
        </>
    );
};
