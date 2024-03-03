import PageContentBlock from '@/components/elements/PageContentBlock';
import React, { Fragment } from 'react';
import FlashMessageRender from '@/components/FlashMessageRender';
import ContentBox from '@/components/elements/ContentBox';
import { Form, Formik, FormikHelpers } from 'formik';
import { object, string, setLocale } from 'yup';
import { pt } from 'yup-locales';
import SpinnerOverlay from '@/components/elements/SpinnerOverlay';
import Field from '@/components/elements/Field';
import { Button } from '@/components/elements/button';
import redeemCoupon from '@/api/account/redeemCoupon';
import { Actions, useStoreActions } from 'easy-peasy';
import { ApplicationStore } from '@/state';
import { httpErrorToHuman } from '@/api/http';

const schema = object().shape({
    code: string().required('Você deve especificar o código que deseja resgatar.'),
});

export default () => {
    const { clearFlashes, addFlash } = useStoreActions((actions: Actions<ApplicationStore>) => actions.flashes);

    setLocale(pt);
    const submit = (values: { code: string }, { resetForm, setSubmitting }: FormikHelpers<{ code: string }>) => {
        clearFlashes();
        redeemCoupon(values.code)
            .then(() => {
                addFlash({ type: 'success', key: 'coupons', message: 'Cupom resgatado com sucesso.' });
            })
            .catch((err) => {
                addFlash({ type: 'danger', key: 'coupons', message: httpErrorToHuman(err) });
            })
            .then(() => {
                resetForm();
                setSubmitting(false);
            });
    };

    return (
        <PageContentBlock title={'Cupons'}>
            <h1 className={'text-5xl'}>Cupons</h1>
            <h3 className={'text-2xl mt-2 text-neutral-500'}>Resgatar cupons dados a você.</h3>
            <FlashMessageRender byKey={'coupons'} className={'mt-2'} />
            <ContentBox title={'Resgatar'} className={'w-1/4 mt-6'}>
                <Formik initialValues={{ code: '' }} onSubmit={submit} validationSchema={schema}>
                    {({ isSubmitting, isValid }) => (
                        <Fragment>
                            <SpinnerOverlay size={'large'} visible={isSubmitting} />
                            <Form>
                                <Field id={'code'} type={'text'} name={'code'} label={'Coloque o código'} />
                                <div className={'mt-6'}>
                                    <Button disabled={isSubmitting || !isValid}>Resgatar</Button>
                                </div>
                            </Form>
                        </Fragment>
                    )}
                </Formik>
            </ContentBox>
        </PageContentBlock>
    );
};
